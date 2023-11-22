use std::collections::HashMap;
use std::fs;
use std::time::Instant;

use crate::{config::Config, store::Entry, Error};

const EOF: u8 = 0xFF;
const SELECT_DB: u8 = 0xFE;
const EXPIRE_TIME: u8 = 0xFD;
const EXPIRE_TIME_MS: u8 = 0xFC;
const RESIZE_DB: u8 = 0xFB;
const AUXILLARY_FIELDS: u8 = 0xFA;

#[derive(Debug, PartialEq)]
enum LengthEncodingType {
    Length(usize),
    Special(EncodingFormat),
}

#[derive(Debug, PartialEq)]
enum EncodingFormat {
    Integer(usize),
    Compressed,
}

#[derive(Debug, PartialEq)]
enum Value {
    String = 0,
    List = 1,
    Set = 2,
    SortedSet = 3,
    Hash = 4,
    Zipmap = 9,
    Ziplist = 10,
    Intset = 11,
    SortedSetInZiplist = 12,
    HashmapInZiplist = 13,
    ListInQuicklist = 14,
}

impl TryFrom<u8> for Value {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        println!("---------- try_from : {value}");
        match value {
            0 => Ok(Value::String),
            1 => Ok(Value::List),
            2 => Ok(Value::Set),
            3 => Ok(Value::SortedSet),
            4 => Ok(Value::Hash),
            9 => Ok(Value::Zipmap),
            10 => Ok(Value::Ziplist),
            11 => Ok(Value::Intset),
            12 => Ok(Value::SortedSetInZiplist),
            13 => Ok(Value::HashmapInZiplist),
            14 => Ok(Value::ListInQuicklist),
            _ => Err(Error::InvalidCommand("Unrecognized value for Value type")),
        }
    }
}

pub fn read_rdb_file(config: &Config) -> Option<HashMap<String, Entry>> {
    if let Some(file_path) = config.get_rdb_path() {
        let file = fs::read(file_path);

        match file {
            Ok(file_content) => {
                // println!("{:?}", file_content);
                Some(rdb_parser(&file_content[..]))
            }
            Err(_) => {
                println!("Couldn't find RDB file so skipping reading RDB file content into state");
                None
            }
        }
    } else {
        None
    }
}

fn rdb_parser(data: &[u8]) -> HashMap<String, Entry> {
    if &data[..5] != b"REDIS" {
        panic!("Expected magic string (5 bytes) to have value 'REDIS'");
    }
    println!("[!] parsed MAGIC STRING: read 5 bytes");

    let rdb_version = String::from_utf8_lossy(&data[5..9]);
    println!("[!] RDB file version: {}, read 4 bytes", rdb_version);

    let mut data = &data[9..];
    let mut hm: HashMap<String, Entry> = HashMap::new();

    while !data.is_empty() {
        match data[0] {
            EOF => {
                println!("[!] Reached EOF");
                data = &data[data.len()..]; // exit while loop
            }
            SELECT_DB => {
                println!("[!] Reached SELECT_DB");
                // Skip two bytes: OP_CODE <DB_selector_value>
                // Assuming DB_selector_value is only 1 byte.... should be atleast for our usecase
                let db_selector = data[1];
                println!(
                    "---- DB selector value: {}, parsed bytes = 2 (OPCODE & DB_selector_value)",
                    db_selector
                );
                data = &data[2..];
            }
            RESIZE_DB => {
                println!("[!] Reached RESIZE_DB");
                // Jump over OP_CODE
                // println!("-------------- DEBUG: current byte: {}", data[0]);
                // println!("-------------- DEBUG: second byte: {}", data[1]);
                // println!("-------------- DEBUG: thrid byte: {}", data[2]);

                // data = &data[1..];

                // let (_hash_table_size, bytes_read) = parse_string(&data).unwrap();
                // data = &data[bytes_read..];

                // let (_expire_hash_table_size, bytes_read) = parse_string(&data).unwrap();
                // data = &data[bytes_read..];
                data = &data[3..];
            }
            AUXILLARY_FIELDS => {
                println!("[!] Reached AUXILLARY_FIELDS");
                data = &data[1..];

                let (key, bytes_read) = parse_string(&data).unwrap();
                data = &data[bytes_read..];

                let (value, bytes_read) = parse_string(&data).unwrap();
                data = &data[bytes_read..];

                println!(
                    "===================> INFO: Parsed aux field ----> {} : {}",
                    key, value
                );
            }
            EXPIRE_TIME => {
                // data = &data[1..];

                // // read 4 byte unsigned integer
                // let raw_data = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as u64;
                // data = &data[4..];

                // let expiring_at = std::time::UNIX_EPOCH + raw_data;

                // // read KV
                // let (key, value, parsed_bytes) = read_key_string_value(&data).unwrap();
                // data = &data[parsed_bytes..];
                unimplemented!()
            }
            EXPIRE_TIME_MS => {
                unimplemented!()
            }
            _ => {
                let (key, value, bytes_read) = read_key_string_value(&data).unwrap();
                println!("[!] Read KV pair without expiry ----> {key} : {value}");
                data = &data[bytes_read..];

                let entry = Entry::new(value, None, None, Instant::now());
                hm.insert(key, entry);
            }
        }
    }

    hm
}

fn read_key_string_value(buf: &[u8]) -> Result<(String, String, usize), Error> {
    match Value::try_from(buf[0]).unwrap() {
        Value::String => {
            let mut rest = &buf[1..];
            let mut bytes_read = 1; // 1 cos we read first byte (value type)

            let (key, parsed_bytes) = parse_string(&rest).unwrap();
            bytes_read += parsed_bytes;
            rest = &rest[parsed_bytes..];

            let (value, parsed_bytes) = parse_string(&rest).unwrap();
            bytes_read += parsed_bytes;

            Ok((key, value, bytes_read))
        }
        _ => unimplemented!(),
    }
}

fn parse_string(buf: &[u8]) -> Result<(String, usize), Error> {
    let mut bytes_read: usize = 0;

    let (length_encoding_type, parsed_bytes) = decode_length_encoding(&buf).unwrap();

    let rest = &buf[parsed_bytes..];
    bytes_read += parsed_bytes;

    let data: String = match length_encoding_type {
        LengthEncodingType::Length(length) => {
            // println!("---- Reading {} raw bytes", length);
            let mut parsed_string = String::new();
            if length > 0 {
                // println!("----------- DEBUG raw read value: {:?}", &rest[..length]);
                parsed_string = String::from_utf8_lossy(&rest[..length]).to_string();
                // println!("---- Parsed String: {}", parsed_string);
            }
            bytes_read += length;
            parsed_string
        }
        LengthEncodingType::Special(spl_format) => match spl_format {
            EncodingFormat::Integer(n) => {
                bytes_read += n;
                match n {
                    1 => rest[0].to_string(),
                    2 => u16::from_be_bytes([rest[0], rest[1]]).to_string(),
                    4 => u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]).to_string(),
                    _ => return Err(Error::InvalidCommand("Invalid length encoding type")),
                }
            }
            _ => return Err(Error::InvalidCommand("Invalid length encoding type")),
        },
    };

    // println!(
    // "-------------- DEBUG: Bytes read at end of parsing string: {}",
    //     bytes_read
    // );
    Ok((data, bytes_read))
}

fn decode_length_encoding(buf: &[u8]) -> Result<(LengthEncodingType, usize), Error> {
    let first_byte = buf[0];
    println!("---- length encoding byte: {:b}", first_byte);

    let two_msb_value = first_byte >> 6;
    // println!("---- MSB value: {:b}", two_msb_value);

    match two_msb_value {
        0b00 => Ok((LengthEncodingType::Length((first_byte & 0x3f) as usize), 1)),
        0b01 => {
            let next_byte = buf[1];
            let length = u16::from_be_bytes([(first_byte & 0x3f), next_byte]) as usize;
            Ok((LengthEncodingType::Length(length), 2))
        }
        0b10 => {
            let length = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
            Ok((LengthEncodingType::Length(length), 5))
        }
        0b11 => {
            // Special format
            println!("---- Special format byte: {:b}", first_byte & 0x3f);
            match first_byte & 0x3f {
                0b00 => {
                    // 8 bit Integer
                    Ok((LengthEncodingType::Special(EncodingFormat::Integer(1)), 1))
                }
                0b01 => {
                    // 16 bit integer
                    Ok((LengthEncodingType::Special(EncodingFormat::Integer(2)), 1))
                }
                0b10 => {
                    // 32 bit integer
                    Ok((LengthEncodingType::Special(EncodingFormat::Integer(4)), 1))
                }
                0b11 => {
                    // Compressed
                    Ok((LengthEncodingType::Special(EncodingFormat::Compressed), 1))
                }
                _ => Err(Error::InvalidCommand(
                    "Unexpected value of remaining 6 bits for special format",
                )),
            }
        }
        _ => Err(Error::InvalidCommand("Unexpected value for 2 MSBs")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading_string_kv() {
        assert_eq!(
            read_key_string_value(&[0, 01, 97, 01, 98]).unwrap(),
            (String::from("a"), String::from("b"), 5)
        );

        assert_eq!(
            read_key_string_value(&[0, 03, 102, 111, 111, 04, 98, 97, 114, 115]).unwrap(),
            (String::from("foo"), String::from("bars"), 10)
        );
    }

    #[test]
    fn test_length_encoding() {
        assert_eq!(
            decode_length_encoding(&[09]).unwrap(),
            (LengthEncodingType::Length(9 as usize), 1)
        );

        assert_eq!(
            decode_length_encoding(&[65, 01]).unwrap(),
            (LengthEncodingType::Length(257 as usize), 2)
        );

        assert_eq!(
            decode_length_encoding(&[192]).unwrap(),
            (LengthEncodingType::Special(EncodingFormat::Integer(1)), 1)
        );

        assert_eq!(
            decode_length_encoding(&[193]).unwrap(),
            (LengthEncodingType::Special(EncodingFormat::Integer(2)), 1)
        );

        assert_eq!(
            decode_length_encoding(&[194]).unwrap(),
            (LengthEncodingType::Special(EncodingFormat::Integer(4)), 1)
        );
    }

    #[test]
    fn test_string_parsing() {
        assert_eq!(parse_string(&[01, 97]).unwrap(), ("a".to_string(), 2));
        assert_eq!(parse_string(&[02, 97, 98]).unwrap(), ("ab".to_string(), 3));

        assert_eq!(parse_string(&[192, 01]).unwrap(), ("1".to_string(), 2));
        assert_eq!(
            parse_string(&[193, 01, 00]).unwrap(),
            ("256".to_string(), 3)
        );
        assert_eq!(
            parse_string(&[194, 01, 00, 00, 00]).unwrap(),
            ("16777216".to_string(), 5)
        );
    }

    #[test]
    fn test_foo() {
        assert_eq!(Value::try_from(9 as u8).unwrap(), Value::Zipmap);
    }
}
