use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;

pub struct PacketParser<'a> {
    packet: &'a [u8],
}

impl<'a> PacketParser<'a> {
    pub fn new(packet: &'a [u8]) -> Self {
        PacketParser { packet }
    }

    pub fn parse_traffic(&self) -> Result<String, i32> {
        if let Some(eth_packet) = EthernetPacket::new(self.packet) {
            // println!(
            //     "| ETH [SRC: {:?}] [DST: {:?}]",
            //     eth_packet.get_source(),
            //     eth_packet.get_destination()
            // );
            match eth_packet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(ipv4packet) = Ipv4Packet::new(eth_packet.payload()) {
                        return self.parse_protocol_ipv4(ipv4packet);
                    } else {
                        return Err(-1);
                    }
                }
                _ => {
                    // println!("{}", eth_packet.get_ethertype().to_string().to_uppercase());
                    return Err(-2);
                }
            }
        }

        Err(-1)
    }

    fn parse_protocol_ipv4(&self, ipv4_packet: Ipv4Packet) -> Result<String, i32> {
        // println!(
        //     "| IPV4 [SRC: {:}] [DST: {:?}]",
        //     ipv4_packet.get_source(),
        //     ipv4_packet.get_destination()
        // );
        match ipv4_packet.get_next_level_protocol() {
            IpNextHeaderProtocols::Udp => self.parse_udp(ipv4_packet),
            _ => {
                // println!(
                //     "{}",
                //     ipv4_packet
                //         .get_next_level_protocol()
                //         .to_string()
                //         .to_uppercase()
                // );
                Err(-1)
            }
        }
    }

    fn parse_udp(&self, ipv4_packet: Ipv4Packet) -> Result<String, i32> {
        if let Some(udp_packet) = UdpPacket::new(ipv4_packet.payload()) {
            // println!(
            //     "| UDP [SRC PRT: {:?}] [DST PRT: {:?}]",
            //     udp_packet.get_source(),
            //     udp_packet.get_destination()
            // );
            let payload = udp_packet.payload();
            if let Ok(payload_str) = std::str::from_utf8(payload) {
                // println!("| {}", payload_str);
                return Ok(String::from(payload_str));
            }
        }
        Err(-1)
    }
}
