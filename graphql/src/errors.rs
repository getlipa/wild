use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum GraphQlRuntimeErrorCode {
    AuthServiceError,
    AccessExpired,
    NetworkError,
    GenericError,
    CorruptData,
    ObjectNotFound,
}

impl Display for GraphQlRuntimeErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type Error = perro::Error<GraphQlRuntimeErrorCode>;

pub type Result<T> = std::result::Result<T, perro::Error<GraphQlRuntimeErrorCode>>;
