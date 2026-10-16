use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::udp::UdpPacket;

/// Errors produced while dissecting a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketParseError {
    /// The frame is shorter than the header it claims to contain.
    Truncated,
    /// The Ethernet frame does not carry IPv4.
    UnsupportedEtherType,
    /// The IPv4 packet does not carry UDP.
    UnsupportedTransportProtocol,
    /// The UDP payload is not valid UTF-8.
    InvalidUtf8Payload,
}

impl std::fmt::Display for PacketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            PacketParseError::Truncated => "frame is truncated",
            PacketParseError::UnsupportedEtherType => {
                "unsupported ethertype (only IPv4 is handled)"
            }
            PacketParseError::UnsupportedTransportProtocol => {
                "unsupported transport protocol (only UDP is handled)"
            }
            PacketParseError::InvalidUtf8Payload => "UDP payload is not valid UTF-8",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for PacketParseError {}

pub struct PacketParser<'a> {
    packet: &'a [u8],
}

impl<'a> PacketParser<'a> {
    pub fn new(packet: &'a [u8]) -> Self {
        PacketParser { packet }
    }

    /// Dissects the frame (Ethernet / IPv4 / UDP) and returns the UDP payload as UTF-8.
    /// The returned slice borrows from the original packet, no allocation takes place.
    pub fn parse_traffic(&self) -> Result<&'a str, PacketParseError> {
        let ethernet_packet =
            EthernetPacket::new(self.packet).ok_or(PacketParseError::Truncated)?;
        if ethernet_packet.get_ethertype() != EtherTypes::Ipv4 {
            return Err(PacketParseError::UnsupportedEtherType);
        }

        // EthernetPacket::new validated the frame length, so this slice is in bounds.
        let ipv4_bytes = &self.packet[EthernetPacket::minimum_packet_size()..];
        let ipv4_packet = Ipv4Packet::new(ipv4_bytes).ok_or(PacketParseError::Truncated)?;
        if ipv4_packet.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
            return Err(PacketParseError::UnsupportedTransportProtocol);
        }

        let ipv4_header_length = ipv4_packet.get_header_length() as usize * 4;
        if ipv4_header_length < Ipv4Packet::minimum_packet_size()
            || ipv4_bytes.len() < ipv4_header_length
        {
            return Err(PacketParseError::Truncated);
        }
        let udp_bytes = &ipv4_bytes[ipv4_header_length..];
        if udp_bytes.len() < UdpPacket::minimum_packet_size() {
            return Err(PacketParseError::Truncated);
        }

        let payload = &udp_bytes[UdpPacket::minimum_packet_size()..];
        std::str::from_utf8(payload).map_err(|_| PacketParseError::InvalidUtf8Payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::MutablePacket;
    use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};
    use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
    use pnet::packet::ipv4::MutableIpv4Packet;
    use pnet::packet::udp::MutableUdpPacket;
    use pnet::util::MacAddr;
    use std::net::Ipv4Addr;

    const SRC_MAC: MacAddr = MacAddr(0x54, 0x00, 0x00, 0x00, 0x00, 0x10);
    const DST_MAC: MacAddr = MacAddr(0x54, 0x00, 0x00, 0x00, 0x00, 0x60);

    fn build_frame(payload: &[u8], protocol: IpNextHeaderProtocol) -> Vec<u8> {
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
        ipv4.set_next_level_protocol(protocol);
        ipv4.set_source(Ipv4Addr::new(10, 42, 0, 10));
        ipv4.set_destination(Ipv4Addr::new(10, 42, 0, 60));
        ipv4.set_checksum(0);
        let ipv4_checksum = pnet::packet::ipv4::checksum(&ipv4.to_immutable());
        ipv4.set_checksum(ipv4_checksum);

        let mut udp = MutableUdpPacket::new(ipv4.payload_mut()).unwrap();
        udp.set_source(1111);
        udp.set_destination(8888);
        udp.set_length((8 + payload.len()) as u16);
        udp.set_payload(payload);
        udp.set_checksum(0);

        buffer
    }

    #[test]
    fn parses_udp_payload() {
        let frame = build_frame(b"$IIHDT,33,T*0C", IpNextHeaderProtocols::Udp);
        assert_eq!(
            PacketParser::new(&frame).parse_traffic().unwrap(),
            "$IIHDT,33,T*0C"
        );
    }

    #[test]
    fn rejects_truncated_frame() {
        assert_eq!(
            PacketParser::new(&[0u8; 5]).parse_traffic(),
            Err(PacketParseError::Truncated)
        );
    }

    #[test]
    fn rejects_non_ipv4_ethertype() {
        let mut buffer = [0u8; 14];
        let mut ethernet = MutableEthernetPacket::new(&mut buffer).unwrap();
        ethernet.set_ethertype(EtherTypes::Arp);

        assert_eq!(
            PacketParser::new(&buffer).parse_traffic(),
            Err(PacketParseError::UnsupportedEtherType)
        );
    }

    #[test]
    fn rejects_non_udp_transport() {
        let frame = build_frame(b"payload", IpNextHeaderProtocols::Tcp);
        assert_eq!(
            PacketParser::new(&frame).parse_traffic(),
            Err(PacketParseError::UnsupportedTransportProtocol)
        );
    }

    #[test]
    fn rejects_non_utf8_payload() {
        let frame = build_frame(&[0xFF, 0xFE, 0xFD], IpNextHeaderProtocols::Udp);
        assert_eq!(
            PacketParser::new(&frame).parse_traffic(),
            Err(PacketParseError::InvalidUtf8Payload)
        );
    }
}
