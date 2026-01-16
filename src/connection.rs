use bytes::{Buf, Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::frame::Frame;

const BUFFER_CAPACITY: usize = 4096;

pub struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(BUFFER_CAPACITY),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, std::io::Error> {
        loop {
            // Try to parse a frame from the existing buffer
            if let Some(frame) = parse_frame(&mut self.buffer)? {
                return Ok(Some(frame));
            }

            // Read more data from the socket
            let n = self.stream.read_buf(&mut self.buffer).await?;

            if n == 0 {
                // Connection closed
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "connection reset by peer",
                    ));
                }
            }
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), std::io::Error> {
        let mut buf = BytesMut::new();
        serialize_frame(frame, &mut buf);
        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;
        Ok(())
    }
}

fn serialize_frame(frame: &Frame, buf: &mut BytesMut) {
    match frame {
        Frame::Simple(s) => {
            buf.extend_from_slice(b"+");
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        Frame::Error(e) => {
            buf.extend_from_slice(b"-");
            buf.extend_from_slice(e.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        Frame::Integer(i) => {
            buf.extend_from_slice(b":");
            buf.extend_from_slice(i.to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        Frame::Bulk(bytes) => {
            buf.extend_from_slice(b"$");
            buf.extend_from_slice(bytes.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(bytes);
            buf.extend_from_slice(b"\r\n");
        }
        Frame::Null => {
            buf.extend_from_slice(b"$-1\r\n");
        }
        Frame::Array(frames) => {
            buf.extend_from_slice(b"*");
            buf.extend_from_slice(frames.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            for frame in frames {
                serialize_frame(frame, buf);
            }
        }
    }
}

fn parse_frame(buf: &mut BytesMut) -> Result<Option<Frame>, std::io::Error> {
    // Check if we have at least one byte to read
    if buf.is_empty() {
        return Ok(None);
    }

    let mut cursor = std::io::Cursor::new(&buf[..]);

    let frame = match cursor.get_u8() {
        b'+' => {
            let line = read_line(&mut cursor)?;
            Frame::Simple(line)
        }
        b'-' => {
            let line = read_line(&mut cursor)?;
            Frame::Error(line)
        }
        b':' => {
            let line = read_line(&mut cursor)?;
            let value = line.parse::<i64>().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid integer")
            })?;
            Frame::Integer(value)
        }
        b'$' => {
            let line = read_line(&mut cursor)?;
            let len: i64 = line.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid bulk length")
            })?;

            if len == -1 {
                Frame::Null
            } else {
                let len = len as usize;

                if cursor.remaining() < len + 2 {
                    return Ok(None); // wait for more data
                }

                let start = cursor.position() as usize;
                let end = start + len;

                let data = Bytes::copy_from_slice(&buf[start..end]);

                cursor.set_position((end + 2) as u64); // skip \r\n

                Frame::Bulk(data)
            }
        }
        b'*' => {
            let line = read_line(&mut cursor)?;
            let count: usize = line.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid array length")
            })?;

            let mut frames = Vec::with_capacity(count);

            for _ in 0..count {
                match parse_frame_inner(&mut cursor, buf)? {
                    Some(f) => frames.push(f),
                    None => return Ok(None),
                }
            }

            Frame::Array(frames)
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unknown RESP type",
            ))
        }
    };

    let consumed = cursor.position() as usize;
    buf.advance(consumed);

    Ok(Some(frame))
}

fn parse_frame_inner(
    cursor: &mut std::io::Cursor<&[u8]>,
    buf: &BytesMut,
) -> Result<Option<Frame>, std::io::Error> {
    let start = cursor.position() as usize;
    let slice = &buf[start..];

    let mut tmp = BytesMut::from(slice);

    match parse_frame(&mut tmp)? {
        Some(frame) => {
            let consumed = slice.len() - tmp.len();
            cursor.set_position((start + consumed) as u64);
            Ok(Some(frame))
        }
        None => Ok(None),
    }
}

fn read_line(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, std::io::Error> {
    let start = cursor.position() as usize;
    let buf = cursor.get_ref();

    for i in start..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            let line = String::from_utf8(buf[start..i].to_vec()).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid utf-8")
            })?;
            cursor.set_position((i + 2) as u64);
            return Ok(line);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::WouldBlock,
        "incomplete frame",
    ))
}
