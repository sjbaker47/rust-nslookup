mod name;

use std::str;
use std::io::{Read, BufRead, Write, Cursor, Seek, SeekFrom, Result};
use std::net::{Ipv4Addr, UdpSocket, ToSocketAddrs};

use self::name::NameRef;

use rand;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug)]
pub struct DnsQuery {
    header: DnsHeader,
    question: QueryQuestion,
}

#[derive(Debug)]
struct DnsResponse {
    header: DnsHeader,
    question: ResponseQuestion,
    record: ResourceRecord,
}

#[derive(Debug)]
struct DnsHeader {
    transaction_id: u16,
    flags: u16,
    question_rr_count: u16,
    answer_rr_count: u16,
    authority_rr_count: u16,
    additional_rr_count: u16,
}

#[derive(Debug)]
struct QueryQuestion {
    name: String,
    qtype: u16,
    qclass: u16,
}

#[derive(Debug)]
struct ResponseQuestion {
    name: NameRef,
    qtype: u16,
    qclass: u16,
}

#[derive(Debug)]
struct ResourceRecord {
    name: NameRef,
    rr_type: u16,
    rr_class: u16,
    ttl: u32,
    length: u16,
    data: u32,
}

impl DnsQuery {
    pub fn addr_query(domain : String) -> DnsQuery {
        DnsQuery { 
            header: DnsHeader {
                transaction_id: rand::random::<u16>(),
                //Set the recursive bit
                flags: 0x0100,
                question_rr_count: 1,
                answer_rr_count: 0,
                authority_rr_count: 0,
                additional_rr_count: 0,
            },
            question: QueryQuestion {
                name: domain,
                qtype: 1,
                qclass: 1,
            },
        }
    }

    pub fn send_to<A : ToSocketAddrs>(&self, socket : &UdpSocket, addr : A) {
        let packet = self.encode_packet();
        socket.send_to(&packet, addr).unwrap();
        let mut recvbuf = [0; 512];
        let (nbytes, raddr) = socket.recv_from(&mut recvbuf[..]).unwrap();
        let packet = DnsQuery::decode_packet(&recvbuf[..nbytes]).unwrap();
    }

    fn encode_packet(&self) -> Vec<u8> {
        println!("Encoding packet: {:?}", self);
        let mut buffer = vec![];
        self.header.encode(&mut buffer).unwrap();
        self.question.encode(&mut buffer).unwrap();
        buffer
    }

    fn decode_packet(buf: &[u8]) -> Result<DnsResponse> {
        let mut rdr = Cursor::new(&buf);
        let header = DnsHeader::parse(&mut rdr)?;
        println!("Decoded {:?}", header);

        let position = rdr.position();
        let question = ResponseQuestion::parse(&mut rdr, position as u16)?;
        println!("Decoded {:?}", question);

        let position = rdr.position();
        let record = ResourceRecord::parse(&mut rdr, position as u16)?;
        println!("Decoded {:?}", record);

        let ip = Ipv4Addr::from(record.data);
        println!("{}", ip);

        Ok(DnsResponse {
            header: header,
            question: question,
            record: record
        })
    }
}

impl DnsHeader {
    fn parse<R : Read>(rdr: &mut R) -> Result<DnsHeader> {
        Ok(DnsHeader {
            transaction_id: rdr.read_u16::<NetworkEndian>()?,
            flags: rdr.read_u16::<NetworkEndian>()?,
            question_rr_count: rdr.read_u16::<NetworkEndian>()?,
            answer_rr_count: rdr.read_u16::<NetworkEndian>()?,
            authority_rr_count: rdr.read_u16::<NetworkEndian>()?,
            additional_rr_count: rdr.read_u16::<NetworkEndian>()?,
        })
    }

    fn encode(&self, buffer : &mut Vec<u8>) -> Result<()> {
        buffer.write_u16::<NetworkEndian>(self.transaction_id)?;
        buffer.write_u16::<NetworkEndian>(self.flags)?;
        buffer.write_u16::<NetworkEndian>(self.question_rr_count)?;
        buffer.write_u16::<NetworkEndian>(self.answer_rr_count)?;
        buffer.write_u16::<NetworkEndian>(self.authority_rr_count)?;
        buffer.write_u16::<NetworkEndian>(self.additional_rr_count)?;
        Ok(())
    }
}

impl QueryQuestion {
    fn encode(&self, buffer : &mut Vec<u8>) -> Result<()> {
        let encoded_name = name::encode_name(&self.name);
        buffer.write_all(&encoded_name)?;
        buffer.write_u16::<NetworkEndian>(self.qtype)?;
        buffer.write_u16::<NetworkEndian>(self.qclass)?;
        Ok(())
    }
}

impl ResponseQuestion {
    fn parse<R : Read + Seek + BufRead>(rdr: &mut R, position : u16) -> Result<ResponseQuestion> {
        let name = NameRef::parse_reader(rdr, position)?;
        let question = ResponseQuestion {
            name: name,
            qtype: rdr.read_u16::<NetworkEndian>()?,
            qclass: rdr.read_u16::<NetworkEndian>()?,
        };
        Ok(question)
    }
}

impl ResourceRecord {
    fn parse<R : Read + Seek + BufRead>(rdr: &mut R, position : u16) -> Result<ResourceRecord> {
        let name = NameRef::parse_reader(rdr, position)?;
        Ok(ResourceRecord {
            name: name,
            rr_type: rdr.read_u16::<NetworkEndian>()?,
            rr_class: rdr.read_u16::<NetworkEndian>()?,
            ttl: rdr.read_u32::<NetworkEndian>()?,
            length: rdr.read_u16::<NetworkEndian>()?,
            data: rdr.read_u32::<NetworkEndian>()?,
        })
    }
}

