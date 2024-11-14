use hmac::digest::InvalidLength;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Mongo error: {:?}", .0)]
    MongoError(#[from] mongodb::error::Error),
    #[error("Length of signing key is invalid")]
    InvalidKeyLength(#[from] InvalidLength),
    #[error("Failed to sign a key: {:?}", .0)]
    SignFailed(#[from] jwt::Error),
    #[error("Invalid Authorization header")]
    InvalidAuthorizationHeader,
}
