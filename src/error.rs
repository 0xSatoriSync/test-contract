use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    
    // Add any other custom errors you like here.
    #[error("InsufficientBalance")]
    InsufficientBalance {},
    
    #[error("InvalidArgument")]
    InvalidArgument { msg: String },
    
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
