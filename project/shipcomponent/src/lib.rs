use nmea::Nmea;
use packet_parser::PacketParser;
use std::sync::Arc;
use std::{collections::VecDeque, os::fd::AsRawFd};
use xdrippi::{utils::interface_name_to_index, BPFRedirectManager, Umem, UmemAllocator, XDPSocket};
mod utils;
use std::time::Instant;
use utils::ship_component_stats::ShipComponentStats;

pub struct ShipComponent<'a> {
    pub name: String,
    pub ifname: String,
    pub mac: String,
    pub ip: String,
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
    pub fn new(
        name: String,
        ifname: String,
        mac: String,
        ip: String,
        sends: Vec<String>,
        receives: Vec<String>,
    ) -> Self {
        // Getting interface index
        let ifindex = interface_name_to_index(ifname.as_str()).unwrap();

        // Setting up umem
        let umem = Umem::new_4k(760000).unwrap();
        let umem = Arc::new(umem);

        // Setting up xsk
        let mut sock = XDPSocket::new(ifindex, 0, umem.clone(), 4096).unwrap();
        let mut bpf_manager = BPFRedirectManager::attach(ifindex);
        bpf_manager.add_redirect(0, sock.as_raw_fd());

        // setting up the memory allocator for the rings
        let umem_allocator = UmemAllocator::for_umem(umem.clone());

        // setting up the fill ring
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

        ShipComponent {
            name,
            ifname,
            mac,
            ip,
            ifindex,
            bpf_manager,
            sock,
            umem_allocator,
            poll_fd,
            sends,
            receives,
            stats,
        }
    }

    pub fn consume_rx_ring(
        &mut self,
        poll_fd_index: usize,
        poll_fds_len: usize,
        ship_traffic: &mut VecDeque<(usize, Vec<u8>, bool, String)>,
        ship_switch: &mut hashbrown::HashMap<[u8; 6], usize>,
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

        //Parse the incoming message
        let packet_parser = PacketParser::new(rx_slice);
        let mut message_ok: bool = true;
        let mut is_nmea: bool = false;
        let mut prefix: String = String::from("NONMEA");

        //trace stats
        self.stats.total_bytes_sent += rx_slice.len() as f64;
        self.stats.performance_send.push((
            start_time.elapsed().as_secs_f64(),
            ((self.stats.total_bytes_sent * 8.0) / start_time.elapsed().as_secs_f64()) / 1000000.0,
        ));

        match packet_parser.parse_traffic() {
            Ok(message) => (message_ok, is_nmea, prefix) = self.apply_policy(message),
            Err(_) => {}
        }

        if message_ok {
            self.handle_network(
                rx_slice,
                ship_switch,
                &poll_fd_index,
                &poll_fds_len,
                ship_traffic,
                is_nmea,
                prefix,
            );
        } else {
            println!("|-- MESSAGE IS NOT A NMEA SENTENCE OR IS NOT ALLOWED ");
            println!("|-- REC ALLOWED {:?}", self.receives);
            println!("|-- SND ALLOWED {:?}", self.sends);
            println!("----------------------------------------------------")
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

    fn handle_network(
        &self,
        rx_slice: &[u8],
        ship_switch: &mut hashbrown::HashMap<[u8; 6], usize>,
        poll_fd_index: &usize,
        poll_fds_len: &usize,
        ship_traffic: &mut VecDeque<(usize, Vec<u8>, bool, String)>,
        is_nmea: bool,
        prefix: String,
    ) {
        // Update the ship switch and add the packets to the ship traffic
        let eth_dst_addr: &[u8; 6] = &rx_slice[0..6].try_into().unwrap();
        let eth_src_addr: &[u8; 6] = &rx_slice[6..12].try_into().unwrap();

        // Add mac src address to the ship switch
        if !ship_switch.contains_key(eth_src_addr) {
            ship_switch.insert(*eth_src_addr, *poll_fd_index);
            println!("SWITCH_STATUS:\n{:x?}", ship_switch);
        }

        if let Some(destination_poll_fd_index) = ship_switch.get(eth_dst_addr) {
            ship_traffic.push_back((
                *destination_poll_fd_index,
                rx_slice.to_vec(),
                is_nmea,
                prefix,
            ));
        } else {
            // nmea sentences should not be flooded since we want that only the correct recipients get what they expect
            if !is_nmea {
                for j in 0..*poll_fds_len {
                    if *poll_fd_index == j {
                        continue;
                    }
                    ship_traffic.push_back((j, rx_slice.to_vec(), is_nmea, prefix.clone()));
                }
            }
        }
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

    fn apply_policy(&self, message: String) -> (bool, bool, String) {
        // here we know that the message is udp and it should be parsed to check for nmea compliance
        let mut nmea = Nmea::new();
        let message_ok = nmea.parse(message.clone());

        match message_ok {
            Ok(()) => {
                // message is valid nmea
                // now gotta check if the message can be sent by the component
                // nmea.show();
                let prefix = format!("${}{}", nmea.str_talker_id(), nmea.str_sentence_type());
                let is_allowed = self
                    .sends
                    .iter()
                    .any(|allowed_message| prefix == *allowed_message);
                (is_allowed, true, prefix)
            }

            Err(_) => (true, false, String::from("NONMEA")),
        }
    }
}
