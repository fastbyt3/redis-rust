pub mod de;
pub mod se;
pub mod store;

use std::sync::{Arc, RwLock};

use crate::de::StreamDeserializer;
use se::StreamSerializer;
use store::Store;
use tokio::net::TcpStream;

const CRLF: &str = "\r\n";

const INTEGER_PREFIX: char = ':';
const SIMPLE_STRING_PREFIX: char = '+';
const BULK_STRING_PREFIX: char = '$';
const ARRAY_PREFIX: char = '*';
const ERROR_PREFIX: char = '-';
const NULL_PREFIX: char = '_';

#[derive(Debug, PartialEq)]
pub enum Value {
    None,
    SimpleString(String),
    BulkString(String),
    Array(Vec<Value>),
    Integer(i64),
}

impl Value {
    pub fn str_value(&self) -> Option<&str> {
        match self {
            Self::SimpleString(s) | Self::BulkString(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid command received: {0}")]
    InvalidCommand(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    PING,
    ECHO,
    GET,
    SET,
}

impl Command {
    pub fn from_str(s: &str) -> Result<Command, Error> {
        match s.to_lowercase().as_str() {
            "ping" => Ok(Command::PING),
            "echo" => Ok(Command::ECHO),
            _ => Err(Error::InvalidCommand(
                "Invalid command / Command has not been implemented",
            )),
        }
    }

    pub fn construct_response(
        &self,
        request_content: Vec<Value>,
        store: Arc<RwLock<Store>>,
    ) -> Result<Value, Error> {
        match self {
            Command::PING => match request_content.len() {
                1 => Ok(Value::SimpleString("PONG".to_string())),
                2 => {
                    let pong_value = request_content[1]
                        .str_value()
                        .ok_or(Error::InvalidCommand(
                            "Argument passed to PING should be a STRING.",
                        ))?
                        .to_string();

                    Ok(Value::BulkString(pong_value))
                }
                _ => return Err(Error::InvalidCommand("Expected either 0 or 1 arguments")),
            },
            Command::ECHO => {
                if request_content.len() != 2 {
                    return Err(Error::InvalidCommand(
                        "ECHO cmd requires exactly 1 argument.",
                    ));
                }
                let echo_content = request_content[1]
                    .str_value()
                    .ok_or(Error::InvalidCommand(
                        "Argument to ECHO couldn't be parsed as string",
                    ))?
                    .to_string();

                Ok(Value::BulkString(echo_content))
            }
            Command::SET => {
                if request_content.len() != 3 {
                    return Err(Error::InvalidCommand(
                        "SET command expects exactly 2 arguments: KEY and VALUE",
                    ));
                }

                let key = request_content[1].str_value().ok_or(Error::InvalidCommand(
                    "KEY value to SET cmd couldn't be parsed as string",
                ))?;

                let value = request_content[2].str_value().ok_or(Error::InvalidCommand(
                    "VALUE value to SET cmd couldn't be parsed as string",
                ))?;

                let mut guard = store.write().unwrap();
                guard.insert(key.to_string(), value.to_string());

                Ok(Value::SimpleString("OK".to_string()))
            }
            Command::GET => todo!(),
        }
    }
}

pub async fn handle_stream(mut stream: TcpStream, store: Arc<RwLock<Store>>) -> Result<(), Error> {
    let (read, write) = stream.split();
    let mut input_deserializer = StreamDeserializer::new(read);
    let mut output_serializer = StreamSerializer::new(write);

    loop {
        let store = store.clone();
        let request = input_deserializer.decode_next().await?;

        match request {
            Value::Array(data) => {
                if data.len() < 1 {
                    return Err(Error::InvalidCommand(
                        "Expected an array but received 0 bytes",
                    ));
                }

                let cmd_part = data[0].str_value().ok_or(Error::InvalidCommand(
                    "Expected Command to be parseable as string",
                ))?;
                let command = Command::from_str(cmd_part)?;
                let response = command.construct_response(data, store)?;

                output_serializer.write(response).await?;
            }
            _ => {
                return Err(Error::InvalidCommand(
                    "Unrecognizable request..... Type of Value doesnt exist....",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, io::Cursor};

    use tokio::runtime::Runtime;

    use super::*;

    fn run_async_tests<F: Future>(f: F) {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(f);
    }

    async fn decode(data: &[u8]) -> Value {
        StreamDeserializer::new(Cursor::new(data))
            .decode_next()
            .await
            .unwrap()
    }

    async fn encode(value: Value) -> Vec<u8> {
        let buffer = Cursor::new(Vec::new());
        let mut serializer = StreamSerializer::new(buffer);
        serializer.write(value).await.unwrap();
        serializer.into_inner().into_inner()
    }

    #[test]
    fn test_write() {
        run_async_tests(async {
            assert_eq!(
                &encode(Value::SimpleString("foo".into())).await,
                b"+foo\r\n"
            );

            assert_eq!(
                &encode(Value::BulkString("foobar".into())).await,
                b"$6\r\nfoobar\r\n"
            );
        })
    }

    #[test]
    fn test_read() {
        run_async_tests(async {
            assert_eq!(
                decode(b"*2\r\n$3\r\nfoo\r\n$4\r\nbars\r\n").await,
                Value::Array(vec![
                    Value::BulkString("foo".to_string()),
                    Value::BulkString("bars".to_string()),
                ])
            );

            assert_eq!(
                decode(b"+hello\r\n").await,
                Value::SimpleString("hello".to_string())
            );

            assert_eq!(decode(b":10\r\n").await, Value::Integer(10 as i64))
        })
    }

    // #[test]
    // fn test_command() {
    //     assert_eq!(Command::from_str("ping").unwrap(), Command::PING);
    //     assert_eq!(Command::from_str("PING").unwrap(), Command::PING);
    //     assert_eq!(Command::from_str("Ping").unwrap(), Command::PING);

    //     assert_eq!(
    //         Command::PING
    //             .construct_response(vec![Value::BulkString("PING".to_string())])
    //             .await
    //             .unwrap(),
    //         Value::SimpleString("PONG".to_string())
    //     );

    //     assert_eq!(
    //         Command::PING
    //             .construct_response(vec![
    //                 Value::BulkString("PING".to_string()),
    //                 Value::SimpleString("foobar".to_string())
    //             ])
    //             .await
    //             .unwrap(),
    //         Value::BulkString("foobar".to_string())
    //     );

    //     assert_eq!(
    //         Command::ECHO
    //             .construct_response(vec![
    //                 Value::BulkString("ECHO".to_string()),
    //                 Value::SimpleString("foobar".to_string())
    //             ])
    //             .await
    //             .unwrap(),
    //         Value::BulkString("foobar".to_string())
    //     );
    // }
}
