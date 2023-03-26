use std::string;

use crate::{codecs::message, error::MessageParseError, response::Response};

//use macros;

//#[macros::stringlike]

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    /* User registration */
    PASS(String),
    NICK(String, Option<i32>),
    USER(String, String, String, String),

    /* Recipient, Message, cc's */
    PRIVMSG(String, String, Option<Vec<String>>),
    NOTICE(String, String),
    PING(String, Option<String>),
    PONG(String, Option<String>),
    RAW(String)
}

#[allow(non_snake_case)]
impl Command {
    pub fn Pass<S: Into<String>>(password: S) -> Command {
        Command::PASS(password.into())
    }
    pub fn Nick<S: Into<String>>(nick: S, hops: Option<i32>) -> Command {
        Command::NICK(nick.into(), hops.map(|n| n.into()))
    }
    
    pub fn User<S: Into<String>>(user: S, host: S, server: S, real: S) -> Command {
        Command::USER(user.into(), host.into(), server.into(), real.into())
    }

    pub fn Privmsg<S: Into<String>>(nick: S, message: S, cc: Option<Vec<S>>) -> Command {
        Command::PRIVMSG(
            nick.into(),
            message.into(),
            cc.map(|o| o.into_iter().map(|s: S| s.into()).collect()),
        )
    }
    pub fn Notice<S: Into<String>>(nick: S, message: S) -> Command {
        Command::NOTICE(nick.into(), message.into())
    }
    pub fn Ping<S: Into<String>>(target: S, target2: Option<S>) -> Command {
        Command::PING(target.into(), target2.map(|s| s.into()))
    }
    pub fn Pong<S: Into<String>>(target: S, target2: Option<S>) -> Command {
        Command::PONG(target.into(), target2.map(|s| s.into()))
    }

    pub fn Raw<S: Into<String>>(raw: S) -> Command {
        Command::RAW(raw.into())
    }
}

impl Command {
    pub fn new(command: &str, args: Vec<&str>) -> Result<Command, MessageParseError> {
        let command = command.to_uppercase();
        match command.as_str() {
            /* Registration messages */
            "PASS" => {
                if args.len() == 1 {
                    Ok(Command::Pass(args[0]))
                } else {
                    Err(Response::ErrNeedMoreParams(command).into())
                }
            }
            "NICK" => {
                if args.len() == 1 {
                    Ok(Command::Nick(args[0], None))
                } else if args.len() == 2 {
                    Ok(Command::Nick(args[0], Some(args[1].parse::<i32>().map_err(|_| MessageParseError::InvalidArgumentCount)?)))
                } else {
                    Err(MessageParseError::InvalidArgumentCount)
                }
            }
            "USER" => {
                if args.len() == 4 {
                    Ok(Command::User(args[0], args[1], args[2], args[3]))
                } else {
                    Err(Response::ErrNeedMoreParams(command).into())
                }
            }
            "NOTICE" => {
                if args.len() == 2 {
                    Ok(Command::Notice(args[0].to_owned(), args[1].to_owned()))
                } else {
                    Err(Response::ErrNeedMoreParams(command).into())
                }
            }
            "PING" => match args.len() {
                1 => Ok(Command::Ping(args[0].to_owned(), None)),
                2 => Ok(Command::Ping(args[0].to_owned(), Some(args[1].to_owned()))),
                _ => Err(Response::ErrNeedMoreParams(command).into()),
            },
            "PONG" => match args.len() {
                1 => Ok(Command::Pong(args[0].to_owned(), None)),
                2 => Ok(Command::Pong(args[0].to_owned(), Some(args[1].to_owned()))),
                _ => Err(Response::ErrNeedMoreParams(command).into()),
            },
            "PRIVMSG" => match args.len() {
                2 => {
                    if args[0].contains(",") {
                        let ccs: Vec<_> = args[0].split(",").collect();
                        Ok(Command::Privmsg(
                            ccs[0].to_owned(),
                            args[1].to_owned(),
                            Some(ccs[1..].iter().map(|s| s.to_string()).collect()),
                        ))
                    } else {
                        Ok(Command::Privmsg(
                            args[0].to_owned(),
                            args[1].to_owned(),
                            None,
                        ))
                    }
                }
                _ => Err(Response::ErrNeedMoreParams(command).into()),
            },
            _ => Err(Response::ErrNoSuchCommand(command).into()),
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
            Command::PASS(ref password) => stringify("PASSWORD", &[&password]),
            Command::NICK(ref nick, None) => stringify("NICK", &[nick]),
            Command::NICK(ref nick, Some(ref hops)) => stringify("NICK", &[nick, &hops.to_string()]),
            Command::USER(ref u, ref h, ref s, ref r) => stringify("USER", &[u,h,s,r]),
            Command::PRIVMSG(ref recip, ref message, Some(ref ccs)) => stringify(
                "privmsg",
                &[format!("{},{}", recip, ccs.join(",")).as_ref(), &message],
            ),
            Command::PRIVMSG(ref recip, ref message, None) => {
                stringify("PRIVMSG", &[&recip, &message])
            }
            Command::NOTICE(ref nick, ref msg) => stringify("NOTICE", &[&nick, &msg]),
            Command::PING(ref sv1, Some(ref sv2)) => stringify("PING", &[&sv1, &sv2]),
            Command::PING(ref sv1, None) => stringify("PING", &[&sv1]),
            Command::PONG(ref daemon, Some(ref daemon2)) => stringify("PING", &[&daemon, &daemon2]),
            Command::PONG(ref sv1, None) => stringify("PONG", &[&sv1]),
            Command::RAW(ref raw) => stringify(raw, &[]),
        }
    }
}
