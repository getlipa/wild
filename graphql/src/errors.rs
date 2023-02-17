use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum AuthRuntimeErrorCode {
    AuthServiceError,
    NetworkError,
    GenericError,
}

impl Display for AuthRuntimeErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type Error = perro::Error<AuthRuntimeErrorCode>;

pub type Result<T> = std::result::Result<T, perro::Error<AuthRuntimeErrorCode>>;
