extern crate rand;
extern crate byteorder;

use std::io::Write;
use std::net::UdpSocket;
use std::net::ToSocketAddrs;

use byteorder::{NetworkEndian, WriteBytesExt};

struct DnsQuery {
    header: DnsHeader,
    question: DnsQuestion,
}

struct DnsHeader {
    transaction_id: u16,
    flags: u16,
    question_rr_count: u16,
    answer_rr_count: u16,
    authority_rr_count: u16,
    additional_rr_count: u16,
}

struct DnsQuestion {
    name: String,
    qtype: u16,
    qclass: u16,
}

impl DnsQuery {
    fn addr_query(domain : String) -> DnsQuery {
        DnsQuery { 
            header: DnsHeader {
                transaction_id: rand::random::<u16>(),
                flags: 0x0100,
                question_rr_count: 1,
                answer_rr_count: 0,
                authority_rr_count: 0,
                additional_rr_count: 0,
            },
            question: DnsQuestion {
                name: domain,
                qtype: 1,
                qclass: 1,
            },
        }
    }

    fn encode_packet(&self) -> Vec<u8> {
        let mut buffer = vec![];
        buffer.write_u16::<NetworkEndian>(self.header.transaction_id).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.flags).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.question_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.answer_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.authority_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.additional_rr_count).unwrap();

        buffer.write_all(&self.encode_name()).unwrap();
        buffer.write_u16::<NetworkEndian>(self.question.qtype).unwrap();
        buffer.write_u16::<NetworkEndian>(self.question.qclass).unwrap();
        buffer
    }

    //XXX: Only works for ASCII!
    fn encode_name(&self) -> Vec<u8> {
        //Domain names are sent with "length" separators and are null-terminated
        //The domain 'microsoft.com' becomes "0x09microsoft0x03com0x00"
       
        let ref name = self.question.name;
        let parts: Vec<&str> = name.split('.').collect();
        //Need one byte for size of each part, n bytes for the text, and a null byte
        let mut buffer : Vec<u8> = Vec::new();
        for part in parts {
            //Write the size byte first
            buffer.push(part.len() as u8);
            for c in part.chars() {
                //Write each character as ASCII (ignore encoding for now)
                buffer.push(c as u8);
            }
        }
        //Write terminating null byte
        buffer.push(0);
        buffer
    }

    fn send_to<A : ToSocketAddrs>(&self, socket : &UdpSocket, addr : A) {
        let packet = self.encode_packet();
        socket.send_to(&packet, addr).unwrap();
    }
}

fn main() {
    let mut socket = UdpSocket::bind("0.0.0.0:29341").unwrap();
    let request = DnsQuery::addr_query("www.google.com".to_string());
    request.send_to(&mut socket, "8.8.8.8:53");
}
