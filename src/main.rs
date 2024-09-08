use libc::{in_addr, recvfrom, sendto, sockaddr_in, socket, AF_INET, IPPROTO_ICMP, SOCK_RAW};
use std::mem;
use std::net::Ipv4Addr;
use std::os::raw::c_int;
use std::ptr;
use std::time::Instant;

fn create_raw_socket() -> c_int {
    unsafe { socket(AF_INET, SOCK_RAW, IPPROTO_ICMP) }
}

fn create_icmp_echo_request(seq: u16) -> [u8; 8] {
    let mut packet = [0u8; 8];
    packet[0] = 8; // Type
    packet[1] = 0; // Code
    packet[2] = 0; // Checksum (to be filled later)
    packet[3] = 0; // Checksum (to be filled later)
    packet[4] = 0; // Identifier
    packet[5] = 0; // Identifier
    packet[6] = (seq >> 8) as u8; // Sequence number (high byte)
    packet[7] = (seq & 0xFF) as u8; // Sequence number (low byte)

    let cksum = calc_checksum(&packet);
    packet[2] = (cksum >> 8) as u8;
    packet[3] = (cksum & 0xFF) as u8;

    packet
}

fn calc_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    for i in (0..data.len()).step_by(2) {
        let word = (data[i] as u16) << 8 | (data[i + 1] as u16);
        sum = sum.wrapping_add(word as u32);
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

fn send_icmp_echo(socket_fd: c_int, dest_ip: &str, seq: u16) {
    let icmp_packet = create_icmp_echo_request(seq);

    let addr = sockaddr_in {
        sin_family: AF_INET as u8,
        sin_port: 0,
        sin_addr: in_addr {
            s_addr: dest_ip.parse::<Ipv4Addr>().unwrap().into(),
        },
        sin_zero: [0; 8],
        sin_len: 0, // ?
    };

    unsafe {
        sendto(
            socket_fd,
            icmp_packet.as_ptr() as *const _,
            icmp_packet.len(),
            0,
            &addr as *const sockaddr_in as *const _,
            mem::size_of::<sockaddr_in>() as u32,
        );
    }
}

fn recv_icmp_echo(socket_fd: c_int) -> Option<(u16, u128)> {
    let mut buf = [0u8; 1024];

    let start_time = Instant::now();
    unsafe {
        let n = recvfrom(
            socket_fd,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        );

        if n > 0 {
            let elapsed_time = start_time.elapsed().as_millis();

            let seq = ((buf[26] as u16) << 8) | (buf[27] as u16);
            return Some((seq, elapsed_time));
        }
    }

    None
}

fn main() {
    let socket_fd = create_raw_socket();
    let target_ip = "127.0.0.1";

    let seq_num = 1;
    send_icmp_echo(socket_fd, &target_ip, seq_num);

    if let Some((seq, rtt)) = recv_icmp_echo(socket_fd) {
        println!(
            "Received response from {} seq ={} time={} ms",
            target_ip, seq, rtt
        );
    } else {
        println!("Request timed out.");
    }
}
