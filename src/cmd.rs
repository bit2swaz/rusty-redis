use bytes::Bytes;
use crate::frame::Frame;

#[derive(Debug)]
pub enum Command {
    Get { key: String },
    Set { key: String, value: Bytes, expiry_seconds: Option<u64> },
    Ping,
    Subscribe { channel: String },
    Publish { channel: String, message: Bytes },
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
                    if frames.len() < 3 {
                        return Err(ParseError::InvalidFormat(
                            "SET requires at least 2 arguments".to_string()
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

                    let mut expiry_seconds = None;
                    let mut i = 3;
                    while i < frames.len() {
                        match &frames[i] {
                            Frame::Bulk(option) => {
                                let option_str = String::from_utf8_lossy(option).to_uppercase();
                                match option_str.as_str() {
                                    "EX" => {
                                        if i + 1 >= frames.len() {
                                            return Err(ParseError::InvalidFormat(
                                                "EX requires a value".to_string()
                                            ));
                                        }
                                        match &frames[i + 1] {
                                            Frame::Bulk(seconds) => {
                                                let seconds_str = String::from_utf8_lossy(seconds);
                                                expiry_seconds = Some(seconds_str.parse::<u64>().map_err(|_| {
                                                    ParseError::InvalidFormat("EX value must be an integer".to_string())
                                                })?);
                                                i += 2;
                                            }
                                            _ => return Err(ParseError::InvalidFormat("EX value must be a bulk string".to_string())),
                                        }
                                    }
                                    _ => return Err(ParseError::InvalidFormat(format!("unknown option '{}'", option_str))),
                                }
                            }
                            _ => return Err(ParseError::InvalidFormat("option must be bulk string".to_string())),
                        }
                    }

                    Ok(Command::Set { key, value, expiry_seconds })
                }
                "SUBSCRIBE" => {
                    if frames.len() != 2 {
                        return Err(ParseError::InvalidFormat(
                            "SUBSCRIBE requires exactly 1 argument".to_string()
                        ));
                    }

                    let channel = match &frames[1] {
                        Frame::Bulk(bytes) => String::from_utf8_lossy(bytes).to_string(),
                        _ => return Err(ParseError::InvalidFormat("channel must be bulk string".to_string())),
                    };

                    Ok(Command::Subscribe { channel })
                }
                "PUBLISH" => {
                    if frames.len() != 3 {
                        return Err(ParseError::InvalidFormat(
                            "PUBLISH requires exactly 2 arguments".to_string()
                        ));
                    }

                    let channel = match &frames[1] {
                        Frame::Bulk(bytes) => String::from_utf8_lossy(bytes).to_string(),
                        _ => return Err(ParseError::InvalidFormat("channel must be bulk string".to_string())),
                    };

                    let message = match &frames[2] {
                        Frame::Bulk(bytes) => bytes.clone(),
                        _ => return Err(ParseError::InvalidFormat("message must be bulk string".to_string())),
                    };

                    Ok(Command::Publish { channel, message })
                }
                _ => Err(ParseError::InvalidCommand(format!("unknown command '{}'", cmd_name))),
            }
        }
        _ => Err(ParseError::InvalidFormat("command must be an array".to_string())),
    }
}
