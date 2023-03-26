
#[derive(Clone, PartialEq, Debug)]
pub enum Response {
    ErrNotRegistered = 451
}

impl Response {
    pub fn to_string(&self) -> String {
        match self {
            Response::ErrNotRegistered => "451 :You have not registered".to_string()
        }
    }
}

impl<'a> From<&'a Response> for String {

    fn from(value: &'a Response) -> Self {
        match value {
            v => v.to_string()
        }
    }
}