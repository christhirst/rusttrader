use crate::config::ConfigError;
use apca::{api::v2::order::CreateError, RequestError};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use polars::error::PolarsError;

#[derive(thiserror::Error, Debug)]
pub enum CLIError {
    #[error("Database error")]
    Converting,

    #[error("Config error")]
    Config(#[from] ConfigError),

    #[error("Failed to get data from Alpaca API")]
    DB(#[from] apca::RequestError<apca::data::v2::bars::ListError>),

    #[error("Config error")]
    Consfig(#[from] RequestError<CreateError>),

    #[error("Tonic error")]
    Tonic(#[from] tonic::transport::Error),

    #[error("Polars error")]
    Polars(#[from] PolarsError),
}

/* impl From<ConfigError> for CLIError {
    fn from(err: ConfigError) -> CLIError {
        CLIError::ConfigError(err)
    }
}
 */
impl IntoResponse for CLIError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
    }
}

pub type Result<T> = std::result::Result<T, TaError>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TaError {
    /* InvalidParameter,
    DataItemIncomplete,
    DataItemInvalid, */
}

/* impl Display for TaError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            TaError::InvalidParameter => write!(f, "invalid parameter"),
            TaError::DataItemIncomplete => write!(f, "data item is incomplete"),
            TaError::DataItemInvalid => write!(f, "data item is invalid"),
        }
    }
} */

/* impl Error for TaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            TaError::InvalidParameter => None,
            TaError::DataItemIncomplete => None,
            TaError::DataItemInvalid => None,
        }
    }
} */
