use anyhow::{Context, Result};
use hashbrown::HashMap;
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::udp::{MutableUdpPacket, UdpPacket};
use pnet::packet::{MutablePacket, Packet};
use pnet::util::MacAddr;
use shipcomponent::{FilteredPacket, ShipComponent};
use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// How long the simulation monitors the network before shutting down.
const MONITOR_WINDOW: Duration = Duration::from_secs(30);

pub struct Ship<'a> {
    pub components: Vec<ShipComponent<'a>>,
    /// MAC-to-component table, seeded from the policy at startup so that forwarding does not
    /// depend on a receiver having sent traffic first.
    ship_switch: HashMap<[u8; 6], usize>,
}

impl<'a> Ship<'a> {
    pub fn new(components: Vec<ShipComponent<'a>>) -> Result<Self> {
        let mut ship_switch = HashMap::with_capacity(components.len());
        for (index, component) in components.iter().enumerate() {
            ship_switch.insert(component.mac_addr.octets(), index);
        }

        Ok(Ship {
            components,
            ship_switch,
        })
    }

    /// Polls every component's AF_XDP socket, applies the policies and forwards allowed traffic
    /// until [`MONITOR_WINDOW`] has elapsed.
    pub fn monitor_network(&mut self) -> Result<()> {
        let mut poll_fds: Vec<libc::pollfd> = Vec::with_capacity(self.components.len());
        for component in &self.components {
            poll_fds.push(component.poll_fd);
        }

        let start_time = Instant::now();
        loop {
            let ready =
                unsafe { libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as libc::nfds_t, -1) };
            if ready < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error).context("poll on AF_XDP sockets failed");
            }

            // Prepare the structure for the network traffic
            let mut ship_traffic: VecDeque<FilteredPacket> = VecDeque::new();

            for (poll_fd_index, poll_fd) in poll_fds.iter().enumerate() {
                if poll_fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                    eprintln!(
                        "[{}] socket poll error (revents = {:#x})",
                        self.components[poll_fd_index].name, poll_fd.revents
                    );
                    continue;
                }
                if poll_fd.revents & libc::POLLIN == 0 {
                    continue;
                }

                // check every component for traffic to analyse
                let current_component = &mut self.components[poll_fd_index];
                while current_component.sock.rx_ring.can_consume() {
                    current_component.consume_rx_ring(&mut ship_traffic, start_time);
                }
            }

            // send the ship traffic according to each component's policy
            self.send_traffic(&ship_traffic, start_time);

            self.components.iter_mut().for_each(|component| {
                component.refill_umem_allocator();
                component.refill_fill_ring();
            });

            if start_time.elapsed() >= MONITOR_WINDOW {
                return Ok(());
            }
        }
    }

    /// Multicasts every allowed sentence to the components whose receive policy lists its prefix.
    pub fn send_traffic(&mut self, ship_traffic: &VecDeque<FilteredPacket>, start_time: Instant) {
        for packet in ship_traffic {
            self.transmit_multicast(packet, start_time);
        }
    }

    fn transmit_multicast(&mut self, packet: &FilteredPacket, start_time: Instant) {
        for i in 0..self.components.len() {
            if !self.components[i].receives.contains(&packet.prefix) {
                continue;
            }

            let destination_index =
                match self.ship_switch.get(&self.components[i].mac_addr.octets()) {
                    Some(index) => *index,
                    None => continue, // Skip components not registered in the switch
                };

            let new_packet = match Self::forge_packet(
                &packet.data,
                self.components[i].mac_addr,
                self.components[i].ip_addr,
            ) {
                Some(new_packet) if !new_packet.is_empty() => new_packet,
                _ => {
                    eprintln!(
                        "ERROR FORGING PACKET FOR MULTICAST TO [{}]",
                        self.components[i].name
                    );
                    continue;
                }
            };

            self.transmit(destination_index, &new_packet, start_time);
        }
    }

    fn transmit(&mut self, destination_index: usize, data: &[u8], start_time: Instant) {
        let current_component = &mut self.components[destination_index];

        let Some(chunk_index) = current_component.umem_allocator.try_allocate() else {
            eprintln!(
                "| MEMORY ALLOCATION FOR TRANSMISSION TO [{}] FAILED",
                current_component.name
            );
            return;
        };

        // memory for transmission is allocated, needs to be set up
        let tx_offset = current_component
            .sock
            .umem
            .chunk_start_offset_for_index(chunk_index);
        let tx_slice = current_component.sock.tx_ring.get_nth_slice_mut(
            current_component.sock.tx_ring.get_producer_index() as _,
            &current_component.sock.umem,
            Some(tx_offset),
            Some(data.len() as _),
        );

        tx_slice.copy_from_slice(data);
        current_component.sock.tx_ring.advance_producer_index();
        match current_component.sock.wake_for_transmission() {
            Ok(()) => {
                current_component.stats.total_bytes_received += data.len() as f64;
                current_component.stats.performance_receive.push((
                    start_time.elapsed().as_secs_f64(),
                    ((current_component.stats.total_bytes_received * 8.0)
                        / start_time.elapsed().as_secs_f64())
                        / 1_000_000.0,
                ));
            }

            Err(error) => eprintln!(
                "| TRANSMISSION USING SOCK {} FAILED: {}",
                destination_index, error
            ),
        }
    }

    /// Rewrites the destination MAC/IP of an Ethernet/IPv4/UDP frame and recalculates the
    /// IPv4 and UDP checksums.
    fn forge_packet(
        data: &[u8],
        destination_mac: MacAddr,
        destination_ip: Ipv4Addr,
    ) -> Option<Vec<u8>> {
        // Parse the Ethernet packet
        let ethernet_packet = EthernetPacket::new(data)?;
        let mut new_packet_buffer = vec![0u8; data.len()];
        let mut new_ethernet_packet = MutableEthernetPacket::new(&mut new_packet_buffer)?;

        new_ethernet_packet.clone_from(&ethernet_packet);
        new_ethernet_packet.set_destination(destination_mac);

        // Parse the IPv4 payload
        let ipv4_payload = new_ethernet_packet.payload();
        let ipv4_packet = Ipv4Packet::new(ipv4_payload)?;
        if ipv4_packet.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
            return None;
        }
        let mut ipv4_buffer = vec![0u8; ipv4_payload.len()];
        let mut new_ipv4_packet = MutableIpv4Packet::new(&mut ipv4_buffer)?;

        new_ipv4_packet.clone_from(&ipv4_packet);
        new_ipv4_packet.set_destination(destination_ip);

        // Recalculate the IPv4 checksum
        new_ipv4_packet.set_checksum(0); // Reset checksum before calculation
        let checksum = pnet::packet::ipv4::checksum(&new_ipv4_packet.to_immutable());
        new_ipv4_packet.set_checksum(checksum);

        // Parse the UDP payload
        let udp_payload = new_ipv4_packet.payload();
        let udp_packet = UdpPacket::new(udp_payload)?;
        let mut udp_buffer = vec![0u8; udp_payload.len()];
        let mut new_udp_packet = MutableUdpPacket::new(&mut udp_buffer)?;

        new_udp_packet.clone_from(&udp_packet);
        new_udp_packet.set_checksum(0);

        // Calculate the UDP checksum
        let udp_checksum = pnet::packet::udp::ipv4_checksum(
            &new_udp_packet.to_immutable(),
            &new_ipv4_packet.get_source(),
            &new_ipv4_packet.get_destination(),
        );
        new_udp_packet.set_checksum(udp_checksum);

        // Assemble the final packet
        new_ipv4_packet.set_payload(new_udp_packet.packet());
        new_ethernet_packet.set_payload(new_ipv4_packet.packet());

        Some(new_packet_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::packet::ipv4::MutableIpv4Packet;
    use pnet::packet::udp::MutableUdpPacket;

    const PAYLOAD: &[u8] = b"$IIHDT,33,T*0C";
    const SRC_MAC: MacAddr = MacAddr(0x54, 0x00, 0x00, 0x00, 0x00, 0x10);
    const DST_MAC: MacAddr = MacAddr(0x54, 0x00, 0x00, 0x00, 0x00, 0x20);
    const NEW_MAC: MacAddr = MacAddr(0x54, 0x00, 0x00, 0x00, 0x00, 0x60);
    const SRC_IP: Ipv4Addr = Ipv4Addr::new(10, 42, 0, 10);
    const DST_IP: Ipv4Addr = Ipv4Addr::new(10, 42, 0, 20);
    const NEW_IP: Ipv4Addr = Ipv4Addr::new(10, 42, 0, 60);

    fn build_frame(payload: &[u8]) -> Vec<u8> {
        let mut buffer = vec![0u8; 14 + 20 + 8 + payload.len()];

        let mut ethernet = MutableEthernetPacket::new(&mut buffer).unwrap();
        ethernet.set_destination(DST_MAC);
        ethernet.set_source(SRC_MAC);
        ethernet.set_ethertype(EtherTypes::Ipv4);

        let mut ipv4 = MutableIpv4Packet::new(ethernet.payload_mut()).unwrap();
        ipv4.set_version(4);
        ipv4.set_header_length(5);
        ipv4.set_total_length((20 + 8 + payload.len()) as u16);
        ipv4.set_ttl(64);
        ipv4.set_next_level_protocol(IpNextHeaderProtocols::Udp);
        ipv4.set_source(SRC_IP);
        ipv4.set_destination(DST_IP);
        ipv4.set_checksum(0);
        let ipv4_checksum = pnet::packet::ipv4::checksum(&ipv4.to_immutable());
        ipv4.set_checksum(ipv4_checksum);

        let mut udp = MutableUdpPacket::new(ipv4.payload_mut()).unwrap();
        udp.set_source(1111);
        udp.set_destination(8888);
        udp.set_length((8 + payload.len()) as u16);
        udp.set_payload(payload);
        udp.set_checksum(0);
        let udp_checksum = pnet::packet::udp::ipv4_checksum(&udp.to_immutable(), &SRC_IP, &DST_IP);
        udp.set_checksum(udp_checksum);

        buffer
    }

    #[test]
    fn forge_packet_rewrites_addresses_and_checksums() {
        let frame = build_frame(PAYLOAD);

        let forged = Ship::forge_packet(&frame, NEW_MAC, NEW_IP).unwrap();
        assert_eq!(forged.len(), frame.len());

        let ethernet = EthernetPacket::new(&forged).unwrap();
        assert_eq!(ethernet.get_destination(), NEW_MAC);
        assert_eq!(ethernet.get_source(), SRC_MAC);

        let ipv4 = Ipv4Packet::new(ethernet.payload()).unwrap();
        assert_eq!(ipv4.get_destination(), NEW_IP);
        assert_eq!(ipv4.get_source(), SRC_IP);
        assert_eq!(ipv4.get_checksum(), pnet::packet::ipv4::checksum(&ipv4));

        let udp = UdpPacket::new(ipv4.payload()).unwrap();
        assert_eq!(
            udp.get_checksum(),
            pnet::packet::udp::ipv4_checksum(&udp, &ipv4.get_source(), &ipv4.get_destination())
        );
        assert_eq!(udp.payload(), PAYLOAD);
    }

    #[test]
    fn forge_packet_returns_none_for_truncated_input() {
        assert!(Ship::forge_packet(&[0u8; 5], NEW_MAC, NEW_IP).is_none());
    }

    #[test]
    fn forge_packet_returns_none_for_non_udp_payload() {
        let mut frame = build_frame(PAYLOAD);
        // Switch the IPv4 protocol field (byte 23) to TCP.
        frame[23] = 6;
        assert!(Ship::forge_packet(&frame, NEW_MAC, NEW_IP).is_none());
    }
}
