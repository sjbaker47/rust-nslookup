mod name;

use std::str;
use std::io::{Read, Write, Cursor, Seek, SeekFrom};
use std::net::UdpSocket;
use std::net::ToSocketAddrs;

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
struct DnsQueston {
    qtype: u16,
    qclass: u16,
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
enum NameRef {
    Offset(u16),
    Name(String)
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

impl NameRef {
    fn parse(buf: &[u8]) -> NameRef {
        if (buf[0] & 0xc0) == 0xc0 {
            NameRef::Offset(0)
        }
        else {
            NameRef::Name(name::decode_name(buf))
        }
    }

    fn encode(&self) -> Vec<u8> {
        match *self {
            NameRef::Name(ref x) => name::encode_name(x),
            NameRef::Offset(_) => panic!("Can't decode resource pointer yet!"),
        }
    }

    fn encoded_length(&self) -> usize {
        match *self {
            //Length includes: starting length octet,
            //labels with 1-byte lengths (turn into .)
            //and 1-byte null terminator
            NameRef::Name(ref x) => x.len() + 2,
            NameRef::Offset(_) => 2
        }
    }
}

impl DnsQuery {
    pub fn addr_query(domain : String) -> DnsQuery {
        DnsQuery { 
            header: DnsHeader {
                transaction_id: rand::random::<u16>(),
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
        DnsQuery::decode_packet(&recvbuf[..nbytes]);
    }

    fn encode_packet(&self) -> Vec<u8> {
        println!("Encoding packet: {:?}", self);
        let mut buffer = vec![];
        buffer.write_u16::<NetworkEndian>(self.header.transaction_id).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.flags).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.question_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.answer_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.authority_rr_count).unwrap();
        buffer.write_u16::<NetworkEndian>(self.header.additional_rr_count).unwrap();

        let encoded_name = name::encode_name(&self.question.name);
        buffer.write_all(&encoded_name).unwrap();
        buffer.write_u16::<NetworkEndian>(self.question.qtype).unwrap();
        buffer.write_u16::<NetworkEndian>(self.question.qclass).unwrap();
        buffer
    }

    fn decode_packet(buf: &[u8]) {
        let mut rdr = Cursor::new(&buf);
        let header = DnsHeader {
            transaction_id: rdr.read_u16::<NetworkEndian>().unwrap(),
            flags: rdr.read_u16::<NetworkEndian>().unwrap(),
            question_rr_count: rdr.read_u16::<NetworkEndian>().unwrap(),
            answer_rr_count: rdr.read_u16::<NetworkEndian>().unwrap(),
            authority_rr_count: rdr.read_u16::<NetworkEndian>().unwrap(),
            additional_rr_count: rdr.read_u16::<NetworkEndian>().unwrap(),
        };
        println!("Decoded {:?}", header);
        let position = rdr.position() as usize;
        let qnamebuf = &buf[position..];
        let name = NameRef::parse(qnamebuf);
        rdr.seek(SeekFrom::Current(name.encoded_length() as i64)).unwrap();
        let question = ResponseQuestion {
            name: name,
            qtype: rdr.read_u16::<NetworkEndian>().unwrap(),
            qclass: rdr.read_u16::<NetworkEndian>().unwrap(),
        };
        println!("Decoded {:?}", question);

        let position = rdr.position() as usize;
        let rnamebuf = &buf[position..];
        let name = NameRef::parse(rnamebuf);
        rdr.seek(SeekFrom::Current(name.encoded_length() as i64)).unwrap();
        let record = ResourceRecord {
            name: name,
            rr_type: rdr.read_u16::<NetworkEndian>().unwrap(),
            rr_class: rdr.read_u16::<NetworkEndian>().unwrap(),
            ttl: rdr.read_u32::<NetworkEndian>().unwrap(),
            length: rdr.read_u16::<NetworkEndian>().unwrap(),
            data: rdr.read_u32::<NetworkEndian>().unwrap(),
        };
        rdr.seek(SeekFrom::Current(-4)).unwrap();
        let (a, b, c, d) = (
            rdr.read_u8().unwrap(),
            rdr.read_u8().unwrap(),
            rdr.read_u8().unwrap(),
            rdr.read_u8().unwrap());
        assert_eq!(record.length, 4);
        println!("Decoded {:?}", record);
        println!("{}.{}.{}.{}", a, b, c, d);
    }
}
