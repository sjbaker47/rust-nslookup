mod name;

use std::str;
use std::fmt;
use std::io::{Read, BufRead, Write, Cursor, Seek, Result};
use std::net::{Ipv4Addr, Ipv6Addr, UdpSocket, ToSocketAddrs};
use rand;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use self::name::NameRef;

#[derive(Debug)]
pub struct DnsQuery {
    header: DnsHeader,
    question: QueryQuestion,
}

#[derive(Debug)]
pub struct DnsResponse {
    header: DnsHeader,
    question: ResponseQuestion,
    record: Record,
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
struct Record {
    header: RecordHeader,
    payload: RecordPayload,
}

#[derive(Debug)]
struct RecordHeader {
    name: NameRef,
    rr_type: u16,
    rr_class: u16,
    ttl: u32,
    length: u16,
}

#[derive(Debug)]
enum RecordPayload {
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    Other(Vec<u8>),
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
                qtype: 28,
                qclass: 1,
            },
        }
    }

    pub fn send_to<A : ToSocketAddrs>(&self, socket : &UdpSocket, addr : A) {
        let packet = self.encode_packet();
        socket.send_to(&packet, addr).unwrap();
    }

    fn encode_packet(&self) -> Vec<u8> {
        println!("Encoding packet: {:?}", self);
        let mut buffer = vec![];
        self.header.encode(&mut buffer).unwrap();
        self.question.encode(&mut buffer).unwrap();
        buffer.shrink_to_fit();
        buffer
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

impl DnsResponse {
    pub fn recv_from(socket: &UdpSocket) -> Result<DnsResponse> {
        let mut recvbuf = [0; 512];
        let (nbytes, _) = socket.recv_from(&mut recvbuf[..])?;
        DnsResponse::decode_packet(&recvbuf[..nbytes])
    }

    fn decode_packet(buf: &[u8]) -> Result<DnsResponse> {
        let mut rdr = Cursor::new(&buf);
        let header = DnsHeader::parse(&mut rdr)?;
        println!("Decoded {:?}", header);

        let position = rdr.position();
        let question = ResponseQuestion::parse(&mut rdr, position as u16)?;
        println!("Decoded {:?}", question);

        let position = rdr.position();
        let record = Record::parse(&mut rdr, position as u16)?;
        println!("Decoded {:?}", record);

        Ok(DnsResponse {
            header: header,
            question: question,
            record: record
        })
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

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.payload {
            RecordPayload::A(addr) => addr.fmt(f),
            RecordPayload::AAAA(addr) => addr.fmt(f),
            RecordPayload::Other(_) => panic!("Huh?"),
        }
    }
}

impl Record {
    fn parse<R : Read + Seek + BufRead>(rdr: &mut R, position : u16) -> Result<Record> {
        let header = RecordHeader {
            name: NameRef::parse_reader(rdr, position)?,
            rr_type: rdr.read_u16::<NetworkEndian>()?,
            rr_class: rdr.read_u16::<NetworkEndian>()?,
            ttl: rdr.read_u32::<NetworkEndian>()?,
            length: rdr.read_u16::<NetworkEndian>()?,
        };
        let payload = match header.rr_type {
            1 => {
                let rawaddr = rdr.read_u32::<NetworkEndian>()?;
                RecordPayload::A(Ipv4Addr::from(rawaddr))
            },
            28 => {
                let mut rawaddr : [u8; 16]= [0; 16];
                rdr.read_exact(&mut rawaddr)?;
                RecordPayload::AAAA(Ipv6Addr::from(rawaddr))
            },
            _ => {
                let mut buf = Vec::with_capacity(header.length as usize);
                rdr.read_exact(&mut buf)?;
                RecordPayload::Other(buf)
            },
        };
        Ok(Record{
            header: header, 
            payload: payload
        })
    }
}

