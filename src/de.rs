use crate::{Value, ARRAY_PREFIX, BULK_STRING_PREFIX, INTEGER_PREFIX, SIMPLE_STRING_PREFIX};
use std::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;

pub struct StreamDeserializer<S> {
    stream: S,
}

impl<S> StreamDeserializer<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S> StreamDeserializer<S>
where
    S: AsyncRead + Unpin,
{
    async fn read_first_char(&mut self) -> io::Result<char> {
        Ok(self.stream.read_u8().await? as char)
    }

    async fn read_till_end(&mut self, into: &mut String) -> io::Result<()> {
        loop {
            let next = self.stream.read_u8().await? as char;

            if next == '\r' {
                break;
            }
            into.push(next);
        }

        let lf = self.stream.read_u8().await? as char;
        if lf != '\n' {
            panic!("Unexpected terminator after CR");
        }

        Ok(())
    }

    async fn parse_size(&mut self) -> io::Result<Option<usize>> {
        let mut s = String::with_capacity(32);
        self.read_till_end(&mut s).await?;

        Ok(if s == "-1" {
            None
        } else {
            Some(s.parse::<usize>().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Integer value: {} couldn't be parsed, {}", s, e),
                )
            })?)
        })
    }

    async fn decode_simple_string(&mut self, first: Option<char>) -> io::Result<Value> {
        let first_char = match first {
            Some(c) => c,
            None => self.stream.read_u8().await? as char,
        };

        match first_char {
            // bulk string
            BULK_STRING_PREFIX => {
                // find length of string
                if let Some(size) = self.parse_size().await? {
                    let mut buffer = vec![0u8; size];
                    self.stream.read_exact(&mut buffer).await?;

                    let cr = self.stream.read_u8().await? as char;
                    let lf = self.stream.read_u8().await? as char;

                    if cr != '\r' {
                        panic!("Expected CR at end of string");
                    }
                    if lf != '\n' {
                        panic!("Expected LF at end of string after \\r");
                    }

                    let str_value = String::from_utf8(buffer).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Couldn't convert u8 buffer into valid string, {}", e),
                        )
                    })?;

                    Ok(Value::BulkString(Some(str_value)))
                } else {
                    Ok(Value::None)
                }
            }
            SIMPLE_STRING_PREFIX => {
                let mut s = String::with_capacity(32);
                self.read_till_end(&mut s).await?;
                Ok(Value::SimpleString(s))
            }
            INTEGER_PREFIX => {
                let mut s = String::with_capacity(32);
                self.read_till_end(&mut s).await?;

                let s_int = s
                    .parse::<i64>()
                    .expect(&format!("Unable to parse string: {} as i64", s));

                Ok(Value::Integer(s_int))
            }
            _ => panic!("Unexpected first character: {}", first_char),
        }
    }

    pub async fn decode_next(&mut self) -> io::Result<Value> {
        let first_char = self.read_first_char().await?;

        Ok(match first_char {
            ARRAY_PREFIX => {
                if let Some(size) = self.parse_size().await? {
                    let mut values = Vec::with_capacity(size);
                    for _ in 0..size {
                        values.push(self.decode_simple_string(None).await?);
                    }
                    Value::Array(values)
                } else {
                    Value::None
                }
            }
            _ => self.decode_simple_string(Some(first_char)).await?,
        })
    }
}
