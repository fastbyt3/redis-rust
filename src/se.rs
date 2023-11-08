use crate::ARRAY_PREFIX;
use crate::{Value, BULK_STRING_PREFIX, CRLF, INTEGER_PREFIX, SIMPLE_STRING_PREFIX};
use std::io;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub struct StreamSerializer<S> {
    stream: S,
}

impl<S> StreamSerializer<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> StreamSerializer<S>
where
    S: AsyncWrite + Unpin,
{
    pub async fn send_term(&mut self) -> io::Result<()> {
        self.stream.write_all(CRLF.as_bytes()).await
    }

    async fn write_simple_string(&mut self, s: String) -> io::Result<()> {
        self.stream.write_u8(SIMPLE_STRING_PREFIX as u8).await?;
        self.stream.write_all(s.as_bytes()).await?;
        self.send_term().await?;
        Ok(())
    }

    async fn write_bulk_string(&mut self, s: String) -> io::Result<()> {
        let content_bytes = s.as_bytes();
        let content_bytes_len = content_bytes.len();

        self.stream.write_u8(BULK_STRING_PREFIX as u8).await?;
        self.stream
            .write_all(format!("{}", content_bytes_len).as_bytes())
            .await?;
        self.send_term().await?;
        self.stream.write_all(content_bytes).await?;
        self.send_term().await?;
        Ok(())
    }

    async fn write_empty_bulk_string(&mut self) -> io::Result<()> {
        self.stream.write_u8(BULK_STRING_PREFIX as u8).await?;
        let content = "-1";
        self.stream.write_all(content.as_bytes()).await?;
        self.send_term().await?;
        Ok(())
    }

    async fn write_integer(&mut self, n: i64) -> io::Result<()> {
        self.stream.write_u8(INTEGER_PREFIX as u8).await?;
        self.stream.write_i64(n).await?;
        self.send_term().await?;
        Ok(())
    }

    pub async fn write(&mut self, value: Value) -> io::Result<()> {
        match value {
            Value::SimpleString(s) => self.write_simple_string(s).await?,
            Value::BulkString(opt_s) => match opt_s {
                Some(s) => self.write_bulk_string(s).await?,
                None => self.write_empty_bulk_string().await?,
            },
            Value::Integer(n) => self.write_integer(n).await?,
            Value::Array(elements) => {
                // * {len} CRLF <VALUE> CRLF ...
                self.stream.write_u8(ARRAY_PREFIX as u8).await?;

                self.stream
                    .write_all(format!("{}", elements.len()).as_bytes())
                    .await?;

                self.send_term().await?;

                for element in elements.into_iter() {
                    match element {
                        Value::BulkString(opt_s) => match opt_s {
                            Some(s) => self.write_bulk_string(s).await?,
                            None => self.write_empty_bulk_string().await?,
                        },
                        Value::SimpleString(s) => self.write_simple_string(s).await?,
                        Value::Integer(n) => self.write_integer(n).await?,
                        _ => panic!("Hasnt been implemented"),
                    }
                }
            }
            _ => todo!(),
        }

        Ok(())
    }
}
