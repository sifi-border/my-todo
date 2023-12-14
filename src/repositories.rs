pub mod label;
pub mod todo;

use thiserror::Error;

#[derive(Error, Debug)]
enum RepositoryError {
    #[error("Unexpected Error: [{0}]")]
    Unexpected(String),
    #[error("Not Found Error (id: {0})")]
    NotFound(i32),
    #[error("Duplicate data Error (id: {0})")]
    Duplicate(i32),
}
