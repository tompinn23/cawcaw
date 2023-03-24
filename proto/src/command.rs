use std::string;

use crate::{error::MessageParseError, codecs::message};

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    /* Recipient, Message, cc's */
    Privmsg(String, String, Option<Vec<String>>),
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
            "PONG" => match args.len() {
                1 => Ok(Command::Pong(args[0].to_owned(), None)),
                2 => Ok(Command::Pong(args[0].to_owned(), Some(args[1].to_owned()))),
                _ => Err(MessageParseError::InvalidArgumentCount),
            },
            "PRIVMSG" => match args.len() {
                2 => { if args[0].contains(",") {
                        let ccs: Vec<_> = args[0].split(",").collect();
                        Ok(Command::Privmsg(ccs[0].to_owned(), args[1].to_owned(), Some(ccs[1..].iter().map(|s| s.to_string()).collect())))
                    } else { 
                        Ok(Command::Privmsg(args[0].to_owned(), args[1].to_owned(), None)) 
                    } 
                }
                _ => Err(MessageParseError::InvalidArgumentCount)
            }
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

fn stringify_owned(cmd: &str, args: &[String]) -> String {
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
            Command::Privmsg(ref recip, ref message, Some(ref ccs)) => {
                stringify("PRIVMSG", &[format!("{},{}", recip, ccs.join(",")).as_ref(), &message])
            }
            Command::Privmsg(ref recip, ref message, None) => stringify("PRIVMSG", &[&recip, &message]),
            Command::Notice(ref nick, ref msg) => stringify("NOTICE", &[&nick, &msg]),
            Command::Ping(ref sv1, Some(ref sv2)) => stringify("PING", &[&sv1, &sv2]),
            Command::Ping(ref sv1, None) => stringify("PING", &[&sv1]),
            Command::Pong(ref daemon, Some(ref daemon2)) => stringify("PING", &[&daemon, &daemon2]),
            Command::Pong(ref sv1, None) => stringify("PONG", &[&sv1]),
        }
    }
}
