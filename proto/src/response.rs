use std::fmt::format;

#[repr(u32)]
#[derive(Clone, PartialEq, Debug)]
pub enum Response {
    ErrNoSuchCommand(String) = 421,
    ErrNickCollision(String) = 436,
    ErrNotRegistered = 451,
    ErrNeedMoreParams(String) = 461,
}

impl Response {
    pub fn to_string(&self) -> String {
        match self {
            Response::ErrNoSuchCommand(cmd) => format!("421 {} :Unknown command", cmd),
            Response::ErrNickCollision(nick) => format!("436 {} :Nickname collision KILL", nick),
            Response::ErrNotRegistered => "451 :You have not registered".to_string(),
            Response::ErrNeedMoreParams(cmd) => format!("462 {} :Not enough parameters", cmd),
        }
    }
}

impl<'a> From<&'a Response> for String {
    fn from(value: &'a Response) -> Self {
        match value {
            v => v.to_string(),
        }
    }
}
