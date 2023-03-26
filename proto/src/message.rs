use super::command;
use super::response;
use crate::error::{MessageParseError, ProtocolError};
use crate::prefix::Prefix;
use std::{fmt::Write, str::FromStr};

#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub enum MessageContents {
    Command(command::Command),
    Response(response::Response)
}

impl MessageContents {
    pub fn to_string(&self) -> String {
        match self {
            MessageContents::Command(command) => { 
                let mut ret = String::new();
                let cmd: String = From::from(command);
                //TODO: Move to config or somthing i don't know.
                ret.push_str(&cmd);
                ret.push_str("\r\n");
                ret
            },
            MessageContents::Response(response) => {
                let mut ret = String::new();
                let cmd: String = From::from(response);
                //TODO: Move to config or somthing i don't know.
                ret.push_str(&cmd);
                ret.push_str("\r\n");
                ret
            }
            _ => "".to_owned(),
        }
    }
}
#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub prefix: Option<Prefix>,
    pub contents: MessageContents
}

impl Message {
    pub fn new(
        prefix: Option<&str>,
        command: &str,
        args: Vec<&str>,
    ) -> Result<Message, MessageParseError> {
        Ok(Message {
            prefix: prefix.map(|p| p.into()),
            contents: MessageContents::Command(command::Command::new(command, args)?),
        })
    }

    pub fn set_prefix(&mut self, pf: &str) {
        self.prefix = Some(Prefix::from(pf));
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        if let Some(ref prefix) = self.prefix {
            write!(ret, ":{} ", prefix).unwrap();
        }
        ret.push_str(&self.contents.to_string());
        ret.push_str("\r\n");
        ret
            
    }
}



impl From<command::Command> for Message {
    fn from(value: command::Command) -> Self {
        Message {
            prefix: None,
            contents: MessageContents::Command(value),
        }
    }
}

impl From<response::Response> for Message {
    fn from(value: response::Response) -> Self {
        Message {
            prefix: None,
            contents: MessageContents::Response(value),
        }
    }
}

impl FromStr for Message {
    type Err = ProtocolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ProtocolError::InvalidMessage {
                string: "empty message".to_string(),
                cause: MessageParseError::EmptyMessage,
            });
        }
        let mut state = s;

        let prefix = if state.starts_with(':') {
            let prefix = state.find(' ').map(|i| &state[1..i]);
            state = state.find(' ').map_or("", |i| &state[i + 1..]);
            prefix
        } else {
            None
        };
        let suffix = if state.contains(" :") {
            let suffix = state.find(" :").map(|i| &state[i + 2..]);
            state = state.find(" :").map_or("", |i| &state[..i + 1]);
            suffix
        } else {
            None
        };

        let command = match state.find(' ').map(|i| &state[..i]) {
            Some(cmd) => {
                state = state.find(' ').map_or("", |i| &state[i + 1..]);
                cmd
            }
            // If there's no arguments but the "command" starts with colon, it's not a command.
            None if state.starts_with(':') => {
                return Err(ProtocolError::InvalidMessage {
                    string: s.to_owned(),
                    cause: MessageParseError::InvalidCommand,
                })
            }
            // If there's no arguments following the command, the rest of the state is the command.
            None => {
                let cmd = state;
                state = "";
                cmd
            }
        };

        let mut args: Vec<_> = state.splitn(14, ' ').filter(|s| !s.is_empty()).collect();
        if let Some(suffix) = suffix {
            args.push(suffix);
        }

        Message::new(prefix, command, args).map_err(|e| ProtocolError::InvalidMessage {
            string: s.to_owned(),
            cause: e,
        })
    }
}
