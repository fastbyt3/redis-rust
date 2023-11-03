use crate::{Value, BULK_STRING_PREFIX, CRLF, SIMPLE_STRING_PREFIX};
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

    pub async fn write(&mut self, value: Value) -> io::Result<()> {
        match value {
            Value::SimpleString(s) => {
                self.stream.write_u8(SIMPLE_STRING_PREFIX as u8).await?;
                self.stream.write_all(s.as_bytes()).await?;
                self.send_term().await?;
            }
            Value::BulkString(s) => {
                let content_bytes = s.as_bytes();
                let content_bytes_len = content_bytes.len();

                self.stream.write_u8(BULK_STRING_PREFIX as u8).await?;

                self.stream
                    .write_all(format!("{}", content_bytes_len).as_bytes())
                    .await?;

                self.send_term().await?;

                self.stream.write_all(content_bytes).await?;

                self.send_term().await?;
            }
            _ => todo!(),
        }

        Ok(())
    }
}
