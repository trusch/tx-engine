#[derive(Debug)]
pub enum Error {
    InvalidArguments,
    IO(std::io::Error),
    Join(tokio::task::JoinError),
    InsufficientFunds,
    AccountLocked,
    NotFound,
}

pub type Result<T> = core::result::Result<T, Error>;

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::InvalidArguments => write!(f, "invalid arguments"),
            Self::InsufficientFunds => write!(f, "insufficient funds"),
            Self::IO(ref e) => write!(f, "io error: {}", e),
            Self::Join(ref e) => write!(f, "join error: {}", e),
            Self::AccountLocked => write!(f, "account locked"),
            Self::NotFound => write!(f, "not found"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Join(err)
    }
}
