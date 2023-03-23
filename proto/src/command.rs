use std::string;

use crate::error::MessageParseError;

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Notice(String, String),
    Ping(String, Option<String>),
    Pong(String, Option<String>),
}

impl Command {
    pub fn new(command: &str, args: Vec<&str>) -> Result<Command, MessageParseError> {
        match command.to_uppercase().as_str() {
            "NOTICE" => {
                if args.len() != 2 {
                    Ok(Command::Notice(args[0].to_owned(), args[1].to_owned()))
                } else {
                    Err(MessageParseError::InvalidArgumentCount)
                }
            }
            "PING" => match args.len() {
                1 => Ok(Command::Ping(args[0].to_owned(), None)),
                2 => Ok(Command::Ping(args[0].to_owned(), Some(args[1].to_owned()))),
                _ => Err(MessageParseError::InvalidArgumentCount),
            },
            _ => Err(MessageParseError::InvalidCommand),
        }
    }
}

fn stringify(cmd: &str, args: &[&str]) -> String {
    match args.split_last() {
        Some((suffix, args)) => {
            let args = args.join(" ");
            let sp = if args.is_empty() { "" } else { " " };
            let co = if suffix.is_empty() || suffix.contains(' ') || suffix.starts_with(':') {
                ":"
            } else {
                ""
            };
            format!("{}{}{} {}{}", cmd, sp, args, co, suffix)
        }
        None => cmd.to_string(),
    }
}

impl<'a> From<&'a Command> for String {
    fn from(cmd: &'a Command) -> String {
        match *cmd {
            Command::Notice(ref nick, ref msg) => stringify("NOTICE", &[&nick, &msg]),
            Command::Ping(ref sv1, Some(ref sv2)) => stringify("PING", &[&sv1, &sv2]),
            Command::Ping(ref sv1, None) => stringify("PING", &[&sv1]),
            Command::Pong(ref daemon, Some(ref daemon2)) => stringify("PING", &[&daemon, &daemon2]),
            Command::Pong(ref sv1, None) => stringify("PONG", &[&sv1]),
        }
    }
}
