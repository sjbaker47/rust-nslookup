use std::str;

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

pub fn decode_name(buf: &[u8]) -> String {
    let mut name = String::new();
    let mut i = 0;
    while buf[i] != 0 {
        if i != 0 {
            name.push('.');
        }
        let nbytes = buf[i] as usize;
        i += 1;
        let slice = &buf[i..i+nbytes].to_vec();
        i += nbytes;
        let part = str::from_utf8(slice).unwrap();
        name += part;
    }
    name
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
