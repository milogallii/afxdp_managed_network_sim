use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

/// UDP load generator sending a valid NMEA sentence as fast as possible.
///
/// The payload keeps the datagram below the MTU (1400 byte UDP payload -> single Ethernet
/// frame), so no IP fragmentation happens and every frame is inspectable by the firewall.
///
/// Usage: sender [TARGET_ADDR] [DURATION_SECONDS] [THREADS]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    let target_addr: SocketAddr = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("10.42.0.60:8888")
        .parse()
        .expect("invalid target address");

    let duration_secs: u64 = args
        .get(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);

    let num_threads: usize = args
        .get(3)
        .and_then(|value| value.parse().ok())
        .unwrap_or(4);

    // Prepare NMEA message: "$IIHDT,33,T*0C" is a syntactically valid HDT sentence
    // (the checksum is verified by the firewall).
    let header = "$IIHDT,33,T*0C";
    let payload_size: usize = 1400;
    let mut buffer = vec![0u8; payload_size];
    buffer[..header.len()].copy_from_slice(header.as_bytes());

    let deadline = Instant::now() + Duration::from_secs(duration_secs);

    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let buffer = buffer.clone();
        handles.push(thread::spawn(move || {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind UDP socket");
            while Instant::now() < deadline {
                if let Err(error) = socket.send_to(&buffer, target_addr) {
                    eprintln!("failed to send packet: {error}");
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("sender thread panicked");
    }
}
