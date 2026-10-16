use anyhow::{Context, Result};
use nmea::Nmea;
use packet_parser::PacketParser;
use pnet::util::MacAddr;
use std::os::fd::AsRawFd;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use std::{collections::VecDeque, net::Ipv4Addr};
use xdrippi::{BPFRedirectManager, Umem, UmemAllocator, XDPSocket, utils::interface_name_to_index};
mod utils;
use utils::ship_component_stats::ShipComponentStats;

/// An NMEA sentence that passed the ingress (sends) policy of its source component,
/// waiting to be forwarded to the components that are allowed to receive it.
#[derive(Debug)]
pub struct FilteredPacket {
    /// The raw Ethernet frame to reforge for every receiver.
    pub data: Vec<u8>,
    /// The "$TTSSS" / "!TTSSS" prefix (talker id + sentence type) used for policy matching.
    pub prefix: String,
}

pub struct ShipComponent<'a> {
    pub name: String,
    pub ifname: String,
    pub mac: String,
    pub ip: String,
    pub mac_addr: MacAddr,
    pub ip_addr: Ipv4Addr,
    pub ifindex: libc::c_uint,
    pub bpf_manager: BPFRedirectManager,
    pub sock: XDPSocket<'a>,
    pub umem_allocator: UmemAllocator,
    pub poll_fd: libc::pollfd,
    pub sends: Vec<String>,
    pub receives: Vec<String>,
    pub stats: ShipComponentStats,
}

impl ShipComponent<'_> {
    /// Creates the AF_XDP socket for `ifname`, attaches the redirecting XDP program and
    /// primes the umem fill ring.
    pub fn new(
        name: String,
        ifname: String,
        mac: String,
        ip: String,
        sends: Vec<String>,
        receives: Vec<String>,
    ) -> Result<Self> {
        let mac_addr = MacAddr::from_str(&mac)
            .with_context(|| format!("[{name}] invalid MAC address '{mac}'"))?;
        let ip_addr = Ipv4Addr::from_str(&ip)
            .with_context(|| format!("[{name}] invalid IP address '{ip}'"))?;

        // Getting interface index
        let ifindex = interface_name_to_index(ifname.as_str())
            .with_context(|| format!("[{name}] network interface '{ifname}' does not exist"))?;

        // Setting up umem
        let umem = Umem::new_4k(760000).context("failed to allocate UMEM")?;
        let umem = Arc::new(umem);

        // Setting up xsk
        let mut sock = XDPSocket::new(ifindex, 0, umem.clone(), 4096)
            .context("failed to create AF_XDP socket")?;
        let mut bpf_manager = BPFRedirectManager::attach(ifindex);
        bpf_manager.add_redirect(0, sock.as_raw_fd());

        // Setting up the memory allocator for the rings
        let umem_allocator = UmemAllocator::for_umem(umem.clone());

        // Setting up the fill ring
        while let Some(chunk_index) = umem_allocator.try_allocate() {
            if sock.fill_ring.can_produce() {
                sock.fill_ring
                    .produce_umem_offset(sock.umem.chunk_start_offset_for_index(chunk_index));
            } else {
                umem_allocator.release(chunk_index);
                break;
            }
        }

        let poll_fd = libc::pollfd {
            fd: sock.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let stats = ShipComponentStats::new();

        Ok(ShipComponent {
            name,
            ifname,
            mac,
            ip,
            mac_addr,
            ip_addr,
            ifindex,
            bpf_manager,
            sock,
            umem_allocator,
            poll_fd,
            sends,
            receives,
            stats,
        })
    }

    /// Returns the policy prefix of `message` if it is a valid NMEA sentence the component is
    /// allowed to send, `None` otherwise. Non-NMEA traffic is denied by default.
    pub fn allowed_prefix(sends: &[String], message: &str) -> Option<String> {
        let mut nmea = Nmea::new();
        nmea.parse(message).ok()?;

        let prefix = format!(
            "{}{}{}",
            nmea.sentence_start(),
            nmea.str_talker_id(),
            nmea.str_sentence_type()
        );

        sends
            .iter()
            .any(|allowed| allowed == &prefix)
            .then_some(prefix)
    }

    /// Consumes one frame from the RX ring, applies the send policy and queues the frame for
    /// forwarding when it is allowed.
    pub fn consume_rx_ring(
        &mut self,
        ship_traffic: &mut VecDeque<FilteredPacket>,
        start_time: Instant,
    ) {
        let rx_descriptor = self
            .sock
            .rx_ring
            .get_nth_descriptor(self.sock.rx_ring.get_consumer_index() as _);

        let rx_slice = self
            .sock
            .rx_ring
            .get_nth_slice(self.sock.rx_ring.get_consumer_index() as _, &self.sock.umem);

        // Trace stats: everything the component emitted, before policy filtering.
        self.stats.total_bytes_sent += rx_slice.len() as f64;
        self.stats.performance_send.push((
            start_time.elapsed().as_secs_f64(),
            ((self.stats.total_bytes_sent * 8.0) / start_time.elapsed().as_secs_f64())
                / 1_000_000.0,
        ));

        // Application layer firewall: only valid NMEA sentences explicitly allowed by the
        // component's send policy are forwarded, everything else is dropped.
        match PacketParser::new(rx_slice).parse_traffic() {
            Ok(message) => match ShipComponent::allowed_prefix(&self.sends, message) {
                Some(prefix) => ship_traffic.push_back(FilteredPacket {
                    data: rx_slice.to_vec(),
                    prefix,
                }),
                None => self.stats.dropped_frames += 1,
            },
            Err(_) => self.stats.dropped_frames += 1,
        }

        // refill allocator or fill ring
        if self.sock.fill_ring.can_produce() {
            self.sock.fill_ring.produce_umem_offset(rx_descriptor.addr);
        } else {
            self.umem_allocator.release_offset(rx_descriptor.addr);
        }

        // advance index
        self.sock.rx_ring.advance_consumer_index();
    }

    pub fn refill_umem_allocator(&mut self) {
        while self.sock.completion_ring.can_consume() {
            let offset = self
                .sock
                .completion_ring
                .get_nth_umem_offset(self.sock.completion_ring.get_consumer_index() as _);
            self.umem_allocator.release_offset(offset);
            self.sock.completion_ring.advance_consumer_index();
        }
    }

    pub fn refill_fill_ring(&mut self) {
        while let Some(chunk_index) = self.umem_allocator.try_allocate() {
            if self.sock.fill_ring.can_produce() {
                self.sock
                    .fill_ring
                    .produce_umem_offset(self.sock.umem.chunk_start_offset_for_index(chunk_index));
            } else {
                self.umem_allocator.release(chunk_index);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed_sends(prefix: &str) -> Vec<String> {
        vec![String::from(prefix)]
    }

    fn talker_sentence(body: &str) -> String {
        let checksum = body.bytes().fold(0u8, |acc, byte| acc ^ byte);
        format!("${}*{:02X}", body, checksum)
    }

    #[test]
    fn allows_listed_nmea_sentence() {
        let sends = allowed_sends("$IIHDT");
        let message = talker_sentence("IIHDT,33,T");
        assert_eq!(
            ShipComponent::allowed_prefix(&sends, &message),
            Some(String::from("$IIHDT"))
        );
    }

    #[test]
    fn denies_nmea_sentence_that_is_not_listed() {
        let sends = allowed_sends("$IIHDT");
        let message =
            talker_sentence("GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(ShipComponent::allowed_prefix(&sends, &message), None);
    }

    #[test]
    fn denies_non_nmea_traffic_by_default() {
        let sends = allowed_sends("$IIHDT");
        assert_eq!(ShipComponent::allowed_prefix(&sends, "hello world"), None);
        assert_eq!(
            ShipComponent::allowed_prefix(&sends, "$IIHDT,33,T*44"),
            None,
            "sentence with an invalid checksum must be denied"
        );
    }

    #[test]
    fn allows_ais_prefix_for_encapsulated_sentences() {
        let sends = allowed_sends("!AIVDM");
        let body = "AIVDM,1,1,,A,15N:;R0P00PD;88MD5MTDwwD7k,0";
        let checksum = body.bytes().fold(0u8, |acc, byte| acc ^ byte);
        let message = format!("!{}*{:02X}", body, checksum);
        assert_eq!(
            ShipComponent::allowed_prefix(&sends, &message),
            Some(String::from("!AIVDM"))
        );
    }
}
