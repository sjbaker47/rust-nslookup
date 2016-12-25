extern crate rand;
extern crate byteorder;

use std::env;
use std::net::UdpSocket;

mod protocol;

fn main() {
    let hostname = env::args().nth(1).expect("Usage: nslookup <hostname>");
    let mut socket = UdpSocket::bind("0.0.0.0:29341").unwrap();
    let request = protocol::DnsQuery::addr_query(hostname);
    request.send_to(&mut socket, "8.8.8.8:53");
}
