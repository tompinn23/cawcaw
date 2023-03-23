use super::command::Command;
use crate::error::{MessageParseError, ProtocolError};
use crate::prefix::Prefix;
use std::{fmt::Write, str::FromStr};

#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub prefix: Option<Prefix>,
    pub command: Command,
}

impl Message {
    pub fn new(
        prefix: Option<&str>,
        command: &str,
        args: Vec<&str>,
    ) -> Result<Message, MessageParseError> {
        Ok(Message {
            prefix: prefix.map(|p| p.into()),
            command: Command::new(command, args)?,
        })
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        if let Some(ref prefix) = self.prefix {
            write!(ret, ":{} ", prefix).unwrap();
        }
        let cmd: String = From::from(&self.command);
        //TODO: Move to config or something i don't know.
        ret.push_str(&cmd);
        ret.push_str("\r\n");
        ret
    }
}

impl From<Command> for Message {
    fn from(value: Command) -> Self {
        Message {
            prefix: None,
            command: value,
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
