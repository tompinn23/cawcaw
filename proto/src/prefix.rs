use core::fmt;
use std::str::FromStr;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Prefix {
    Server(String),
    Nickname(String, String, String),
}

impl Prefix {
    pub fn new_from_str(str: &str) -> Prefix {
        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        enum Active {
            Name,
            User,
            Host,
        }
        let mut name = String::new();
        let mut user = String::new();
        let mut host = String::new();
        let mut active = Active::Name;
        let mut is_server = false;
        for c in str.chars() {
            if c == '.' && active == Active::Name {
                is_server = true;
            }
            match c {
                '!' if active == Active::Name => {
                    is_server = false;
                    active = Active::User;
                }
                '@' if active != Active::Host => {
                    is_server = false;
                    active = Active::Host;
                }
                _ => {
                    match active {
                        Active::Name => &mut name,
                        Active::User => &mut user,
                        Active::Host => &mut host,
                    }
                    .push(c);
                }
            }
        }
        if is_server {
            Prefix::Server(name)
        } else {
            Prefix::Nickname(name, user, host)
        }
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Prefix::Nickname(n, u, h) => {
                let u = if !u.is_empty() {
                    format!("!{}", u)
                } else {
                    "".to_owned()
                };
                let h = if !h.is_empty() {
                    format!("@{}", h)
                } else {
                    "".to_owned()
                };
                write!(f, "{}{}{}", n, u, h)
            }
            Prefix::Server(n) => write!(f, "{}", n),
        }
    }
}

impl FromStr for Prefix {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Prefix::new_from_str(s))
    }
}

impl<'a> From<&'a str> for Prefix {
    fn from(s: &str) -> Self {
        Prefix::new_from_str(s)
    }
}
