use bytes::Bytes;
use crate::frame::Frame;

#[derive(Debug)]
pub enum Command {
    Get { key: String },
    Set { key: String, value: Bytes },
    Ping,
}

#[derive(Debug)]
pub enum ParseError {
    InvalidCommand(String),
    InvalidFormat(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidCommand(msg) => write!(f, "invalid command: {}", msg),
            ParseError::InvalidFormat(msg) => write!(f, "invalid format: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn from_frame(frame: Frame) -> Result<Command, ParseError> {
    match frame {
        Frame::Array(frames) => {
            if frames.is_empty() {
                return Err(ParseError::InvalidFormat("empty command array".to_string()));
            }

            let cmd_name = match &frames[0] {
                Frame::Bulk(bytes) => String::from_utf8_lossy(bytes).to_uppercase(),
                _ => return Err(ParseError::InvalidFormat("command must be bulk string".to_string())),
            };

            match cmd_name.as_str() {
                "PING" => Ok(Command::Ping),
                "GET" => {
                    if frames.len() != 2 {
                        return Err(ParseError::InvalidFormat(
                            "GET requires exactly 1 argument".to_string()
                        ));
                    }

                    let key = match &frames[1] {
                        Frame::Bulk(bytes) => String::from_utf8_lossy(bytes).to_string(),
                        _ => return Err(ParseError::InvalidFormat("key must be bulk string".to_string())),
                    };

                    Ok(Command::Get { key })
                }
                "SET" => {
                    if frames.len() != 3 {
                        return Err(ParseError::InvalidFormat(
                            "SET requires exactly 2 arguments".to_string()
                        ));
                    }

                    let key = match &frames[1] {
                        Frame::Bulk(bytes) => String::from_utf8_lossy(bytes).to_string(),
                        _ => return Err(ParseError::InvalidFormat("key must be bulk string".to_string())),
                    };

                    let value = match &frames[2] {
                        Frame::Bulk(bytes) => bytes.clone(),
                        _ => return Err(ParseError::InvalidFormat("value must be bulk string".to_string())),
                    };

                    Ok(Command::Set { key, value })
                }
                _ => Err(ParseError::InvalidCommand(format!("unknown command '{}'", cmd_name))),
            }
        }
        _ => Err(ParseError::InvalidFormat("command must be an array".to_string())),
    }
}
