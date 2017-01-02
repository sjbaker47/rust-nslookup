extern crate rand;
extern crate byteorder;

mod protocol;

use std::env;
use std::time::Duration;
use std::net::UdpSocket;
use protocol::{DnsQuery, DnsResponse};

fn main() {
    let hostname = env::args().nth(1).expect("Usage: nslookup <hostname>");
    let socket = UdpSocket::bind("0.0.0.0:29341").unwrap();
    let request = DnsQuery::addr_query(hostname);
    request.send_to(&socket, "8.8.8.8:53");

    socket.set_read_timeout(Some(Duration::new(5, 0))).expect("Failed to set socket duration!");
    let response = DnsResponse::recv_from(&socket).unwrap();
}

