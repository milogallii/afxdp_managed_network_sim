// use std::net::{SocketAddr, UdpSocket};
// use std::thread;
// use std::time::Duration;

// fn main() {
//     let target_addr: SocketAddr = "10.42.0.60:8888".parse().expect("Invalid address");
//     let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");

//     //prepare NMEA message
//     let payload_size: usize = 1460;
//     let header = "$IIHDT,33,T*44";
//     let mut buffer = vec![0u8; payload_size]; buffer[..header.len()].copy_from_slice(header.as_bytes());

//     loop {
//         match socket.send_to(&buffer, target_addr) {
//             Ok(_) => {}
//             Err(e) => {
//                 eprintln!("Failed to send packet: {}", e);
//             }
//         }

//         thread::sleep(Duration::from_nanos(1));
//     }
// }

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let target_addr: SocketAddr = "10.42.0.60:8888".parse().expect("Invalid address");

    // Prepare NMEA message
    let payload_size: usize = 5800;
    let header = "$IIHDT,33,T*44";
    let mut buffer = vec![0u8; payload_size];
    buffer[..header.len()].copy_from_slice(header.as_bytes());

    // Number of threads to spawn
    let num_threads = 4;

    // Use an Arc to share the buffer between threads
    let buffer_arc = Arc::new(buffer);

    // Create a vector to hold the thread handles
    let mut handles = vec![];

    for _ in 0..num_threads {
        // Clone the Arc to share the buffer with the new thread
        let buffer_clone = Arc::clone(&buffer_arc);
        let target_addr = target_addr.clone();

        // Spawn a new thread
        let handle = thread::spawn(move || {
            // Create a new UDP socket for each thread
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");

            loop {
                match socket.send_to(&buffer_clone, target_addr) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to send packet: {}", e);
                    }
                }

                // Sleep for a short duration to avoid overwhelming the CPU
                thread::sleep(Duration::from_nanos(1));
            }
        });

        // Store the thread handle
        handles.push(handle);
    }

    // Wait for all threads to finish (they won't in this case, since they run in an infinite loop)
    for handle in handles {
        handle.join().unwrap();
    }
}
