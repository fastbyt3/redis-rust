pub mod config;
pub mod de;
pub mod rdb;
pub mod se;
pub mod store;

use std::{sync::Arc, time::Instant};

use crate::de::StreamDeserializer;
use config::Config;
use se::StreamSerializer;
use store::Store;
use tokio::net::TcpStream;

const CRLF: &str = "\r\n";

const INTEGER_PREFIX: char = ':';
const SIMPLE_STRING_PREFIX: char = '+';
const BULK_STRING_PREFIX: char = '$';
const ARRAY_PREFIX: char = '*';

#[derive(Debug, PartialEq)]
pub enum Value {
    None,
    SimpleString(String),
    BulkString(Option<String>),
    Array(Vec<Value>),
    Integer(i64),
}

impl Value {
    pub fn str_value(&self) -> Option<&str> {
        match self {
            Self::SimpleString(s) => Some(s.as_str()),
            Self::BulkString(opt_s) => match opt_s {
                Some(s) => Some(s.as_str()),
                None => panic!("Unexpected None for BulkString...."),
            },
            _ => None,
        }
    }

    pub fn int_value(&self) -> Option<i64> {
        match self {
            Self::Integer(x) => Some(x.to_owned()),
            Self::SimpleString(s) => match s.parse::<i64>() {
                Ok(x) => Some(x),
                Err(_) => None,
            },
            Self::BulkString(opt_s) => {
                let s = opt_s.as_ref().expect("Unexpected NONE in BulkString");
                Some(s.parse::<i64>().expect("Unable to parse as i64"))
            }
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
    CONFIG,
    KEYS,
}

impl Command {
    pub fn from_str(s: &str) -> Result<Command, Error> {
        match s.to_lowercase().as_str() {
            "ping" => Ok(Command::PING),
            "echo" => Ok(Command::ECHO),
            "set" => Ok(Command::SET),
            "get" => Ok(Command::GET),
            "config" => Ok(Command::CONFIG),
            "keys" => Ok(Command::KEYS),
            _ => Err(Error::InvalidCommand(
                "Invalid command / Command has not been implemented",
            )),
        }
    }

    pub async fn construct_response(
        &self,
        request_content: Vec<Value>,
        store: Arc<Store>,
        config: &Config,
    ) -> Result<Value, Error> {
        let store = store.clone();
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

                    Ok(Value::BulkString(Some(pong_value)))
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

                Ok(Value::BulkString(Some(echo_content)))
            }
            Command::SET => {
                // SET KEY VALUE
                // SET KEY VALUE PX xxx
                if request_content.len() != 3 && request_content.len() != 5 {
                    return Err(Error::InvalidCommand(
                        "SET command expects exactly 2 arguments: KEY and VALUE and one optional PX (set expiry)",
                    ));
                }

                let key = request_content[1].str_value().ok_or(Error::InvalidCommand(
                    "KEY value to SET cmd couldn't be parsed as string",
                ))?;

                let value = request_content[2].str_value().ok_or(Error::InvalidCommand(
                    "VALUE value to SET cmd couldn't be parsed as string",
                ))?;

                let expires_in = if request_content.len() == 5 {
                    if request_content[3]
                        .str_value()
                        .map(|s| s.to_lowercase())
                        .as_ref()
                        .map(|s| s.as_str())
                        != Some("px")
                    {
                        return Err(Error::InvalidCommand(
                            "Expected either PX or px to specify expiry in ms for data",
                        ));
                    }

                    Some(request_content[4].int_value().unwrap() as u64)
                } else {
                    None
                };

                store
                    .insert(key.to_string(), value.to_string(), expires_in, None)
                    .await;

                Ok(Value::SimpleString("OK".to_string()))
            }
            Command::GET => {
                if request_content.len() != 2 {
                    return Err(Error::InvalidCommand(
                        "GET cmd requires exactly1 argument: KEY",
                    ));
                }

                let key = request_content[1].str_value().ok_or(Error::InvalidCommand(
                    "KEY passed for GET cmd couldn;t be parsed as string",
                ))?;

                match store.get(key, Instant::now()).await {
                    Some(v) => Ok(Value::BulkString(Some(v))),
                    None => Ok(Value::BulkString(None)),
                }
            }
            Command::CONFIG => {
                // CONFIG GET
                // CONFIG GET dir
                // CONFIG GET dbfilename
                if request_content.len() != 3 {
                    return Err(Error::InvalidCommand(
                        "CONFIG commands should have exactly 2 arguments. CONFIG GET <CONFIG_KEY>",
                    ));
                }

                if request_content[1]
                    .str_value()
                    .ok_or(Error::InvalidCommand(
                        "Second argument passed couldn't be parsed as String",
                    ))?
                    .to_lowercase()
                    != "get"
                {
                    return Err(Error::InvalidCommand(
                        "CONFIG command expected GET as second arg.",
                    ));
                }

                let key = request_content[2].str_value().ok_or(Error::InvalidCommand(
                    "Error during parsing KEY passed to CONFIG GET",
                ))?;

                match key {
                    "dir" => Ok(Value::Array(vec![
                        Value::BulkString(Some("dir".to_string())),
                        Value::BulkString(Some(
                            config.get_rdb_dir().ok_or("".to_string()).unwrap(),
                        )),
                    ])),
                    "dbfilename" => Ok(Value::Array(vec![
                        Value::BulkString(Some("dbfilename".to_string())),
                        Value::BulkString(Some(
                            config.get_rdb_file().ok_or("".to_string()).unwrap(),
                        )),
                    ])),
                    _ => Err(Error::InvalidCommand("Invalid CONFIG key requested")),
                }
            }
            Command::KEYS => {
                // print all keys in store
                println!("{:?}", request_content);
                if request_content.len() != 2 {
                    panic!("KEYS expects exactly 1 argument");
                }

                if request_content[1].str_value().unwrap() == "*" {
                    // let keys: Vec<Value> = Vec::new();
                    // for key in store.get_all_keys().await {
                    //     keys.append(Value::BulkString(Some(key)))
                    // }
                    let keys = store
                        .get_all_keys()
                        .await
                        .iter()
                        .map(|key| Value::BulkString(Some(key.to_owned())))
                        .collect();
                    Ok(Value::Array(keys))
                } else {
                    Err(Error::InvalidCommand("not implemented yet"))
                }
            }
        }
    }
}

pub async fn handle_stream(
    mut stream: TcpStream,
    store: Arc<Store>,
    config: Config,
) -> Result<(), Error> {
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
                let response = command.construct_response(data, store, &config).await?;

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
                &encode(Value::BulkString(Some("foobar".into()))).await,
                b"$6\r\nfoobar\r\n"
            );

            assert_eq!(&encode(Value::BulkString(None)).await, b"$-1\r\n");
        })
    }

    #[test]
    fn test_read() {
        run_async_tests(async {
            assert_eq!(
                decode(b"*2\r\n$3\r\nfoo\r\n$4\r\nbars\r\n").await,
                Value::Array(vec![
                    Value::BulkString(Some("foo".to_string())),
                    Value::BulkString(Some("bars".to_string())),
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
    //     let store = Arc::new(RwLock::new(Store::new()));

    //     assert_eq!(Command::from_str("ping").unwrap(), Command::PING);
    //     assert_eq!(Command::from_str("PING").unwrap(), Command::PING);
    //     assert_eq!(Command::from_str("Ping").unwris_expiredap(), Command::PING);

    //     assert_eq!(
    //         Command::PING
    //             .construct_response(vec![Value::BulkString("PING".to_string())], store.clone())
    //             .unwrap(),
    //         Value::SimpleString("PONG".to_string())
    //     );

    //     assert_eq!(
    //         Command::PING
    //             .construct_response(
    //                 vec![
    //                     Value::BulkString("PING".to_string()),
    //                     Value::SimpleString("foobar".to_string())
    //                 ],
    //                 store.clone()
    //             )
    //             .unwrap(),
    //         Value::BulkString("foobar".to_string())
    //     );

    //     assert_eq!(
    //         Command::ECHO
    //             .construct_response(
    //                 vec![
    //                     Value::BulkString("ECHO".to_string()),
    //                     Value::SimpleString("foobar".to_string())
    //                 ],
    //                 store.clone()
    //             )
    //             .unwrap(),
    //         Value::BulkString("foobar".to_string())
    //     );

    //     assert_eq!(
    //         Command::SET.construct_response(
    //             vec![
    //                 Value::BulkString("SET".to_string),
    //                 Value::
    //             ]
    //         )
    //     )
    // }
}
