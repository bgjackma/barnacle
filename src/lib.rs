use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;

pub mod factory;
pub mod forward;
pub mod listener;

// TODO: Revise Errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to find route for: {0}")]
    RouteError(String),
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),
    #[error("{0:?}")]
    HyperError(#[from] hyper::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
pub type Request = hyper::Request<Incoming>;
pub type Response = hyper::Response<BoxBody<bytes::Bytes, hyper::Error>>;
