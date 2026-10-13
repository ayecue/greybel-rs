use std::io::{self, BufRead, Write};

use greybel_lsp::Message;

pub struct Transport<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> Transport<R, W> {
    const LENGTH: &str = "content-length:";

    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    pub fn read(&mut self) -> io::Result<Option<Message>> {
        let mut length = None;
        let mut line = String::new();

        loop {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                return Ok(None);
            }

            let header = line.trim_end_matches(['\r', '\n']);

            if header.is_empty() {
                if length.is_some() {
                    break;
                }

                continue;
            }

            if header.len() >= Self::LENGTH.len()
                && header[..Self::LENGTH.len()].eq_ignore_ascii_case(Self::LENGTH)
            {
                let value = header[Self::LENGTH.len()..].trim();

                length = Some(value.parse::<usize>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Content-Length {value}: {error}"),
                    )
                })?);
            }
        }

        let mut body = vec![0; length.unwrap_or_default()];

        self.reader.read_exact(&mut body)?;

        serde_json::from_slice(&body)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn write(&mut self, message: &Message) -> io::Result<()> {
        let body = serde_json::to_vec(message).map_err(io::Error::other)?;

        write!(self.writer, "Content-Length: {}\r\n\r\n", body.len())?;
        self.writer.write_all(&body)?;

        self.writer.flush()
    }
}

#[cfg(test)]
#[path = "tests/transport.rs"]
mod tests;
