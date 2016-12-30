use std::str;
use std::io::{Read, BufRead, Seek, SeekFrom, Error, ErrorKind, Result};
use byteorder::{NetworkEndian, ReadBytesExt};

#[derive(Debug)]
pub enum NameRef {
    //A decoded name with an offset from the UDP packet's data payload
    Name(String, u16),
    //A reference to somewhere else in the payload, in bytes
    Offset(u16),
}

impl NameRef {
    pub fn parse_reader<R : Read + Seek + BufRead>(reader : &mut R, position: u16) -> Result<NameRef> {
        let flag = reader.read_u16::<NetworkEndian>()?;
        let offset_flag = 0xc000u16;
        if (flag & offset_flag) == offset_flag {
            let offset_reference = flag & !(offset_flag);
            Ok(NameRef::Offset(offset_reference))
        }
        else {
            //Undo the two bytes we've read -- they're a part of the domain name!
            reader.seek(SeekFrom::Current(-2))?;
            let name = NameRef::Name(decode_name_rdr(reader)?, position);
            Ok(name)
        }
    }
}

//Domain names are sent with "length" separators and are null-terminated
//The domain 'microsoft.com' becomes "0x09microsoft0x03com0x00"
pub fn encode_name(name : &str) -> Vec<u8> {
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

pub fn decode_name_rdr<R : BufRead>(rdr: &mut R) -> Result<String> {
    let mut buffer: Vec<u8> = Vec::new();
    let total = rdr.read_until(0, &mut buffer)?;
    if total == 0 || buffer[total-1] != 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Domain name appears blank or mangled"));
    }

    let mut name = String::new();
    let mut nbytes = 0;
    for byte in buffer {
        //Null byte means we've hit the last label
        if byte == 0 { 
            break; 
        }

        //If nbytes is zero, we're either staritng out or hit the end of the label
        if nbytes == 0 {
            //Interpret this byte as the length of the next label (e.g. google, com)
            nbytes = byte;
            //Replace length bytes with '.' as long as we're not at the first label
            if !name.is_empty() {
                name.push('.');
            }
        }
        //Positive nbytes mean we're still reading a label
        else {
            //Intepret the byte as an ASCII char
            name.push(byte as char);
            nbytes -= 1;
        }
    }
    Ok(name)
}

#[test]
fn encode_name_encodes() {
    let simple_domain = "ab.c";
    let encoded = encode_name(simple_domain);
    let expected : Vec<u8> = vec![2, 'a' as u8, 'b' as u8, 1, 'c' as u8, 0];
    assert_eq!(expected, encoded);
}

#[test]
fn decode_name_decodes() {
    let simple_domain = "example.com";
    let buf = encode_name(simple_domain);
    let decoded = decode_name(&buf);
    assert_eq!(simple_domain, decoded);

    let domain_with_sub = "site.example.com";
    let buf = encode_name(domain_with_sub);
    let decoded = decode_name(&buf);
    assert_eq!(domain_with_sub, decoded);
}

#[test]
fn decode_name_rdr_decodes() {
    let simple_domain = "example.com";
    let buf = encode_name(simple_domain);
    let decoded = decode_name_rdr(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(simple_domain, decoded);

    let domain_with_sub = "site.example.com";
    let buf = encode_name(domain_with_sub);
    let decoded = decode_name_rdr(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(domain_with_sub, decoded);
}
