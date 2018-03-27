extern crate rand;
extern crate byteorder;

mod protocol;

use std::env;
use std::process;
use std::error::Error;
use std::time::Duration;
use std::net::UdpSocket;
use protocol::{DnsQuery, DnsResponse};

fn main() {
    let hostname = env::args().nth(1).expect("Usage: nslookup <hostname>");
    if let Err(e) = run(hostname) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}

fn run(hostname : String) -> Result<DnsResponse, Box<Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let request = DnsQuery::addr_query(hostname);

    //XXX: Custom DNS server, get default from host?
    request.send_to(&socket, "8.8.8.8:53");

    socket.set_read_timeout(Some(Duration::new(5, 0)))?;
    let response = DnsResponse::recv_from(&socket)?;
    for record in &response.records {
        println!("{}", record);
    }
    Ok(response)
}

