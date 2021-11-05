#[derive(Debug)]
pub enum Error {
    InvalidArguments,
    IOError(std::io::Error),
    JoinError(tokio::task::JoinError),
    InsufficientFunds,
    AccountLocked,
    KVError(kv::Error),
    NotFound,
}

pub type Result<T> = core::result::Result<T, Error>;

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::InvalidArguments => write!(f, "invalid arguments"),
            Self::InsufficientFunds => write!(f, "insufficient funds"),
            Self::IOError(ref e) => write!(f, "io error: {}", e),
            Self::JoinError(ref e) => write!(f, "join error: {}", e),
            Self::AccountLocked => write!(f, "account locked"),
            Self::KVError(ref e) => write!(f, "kv error: {}", e),
            Self::NotFound => write!(f, "not found"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::JoinError(err)
    }
}

impl From<kv::Error> for Error {
    fn from(err: kv::Error) -> Self {
        Self::KVError(err)
    }
}
