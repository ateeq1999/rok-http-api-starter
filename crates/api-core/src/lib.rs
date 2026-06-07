pub mod auth;
pub mod crud;
pub mod db;
pub mod migrations;
pub mod response;
pub mod validator;

pub mod prelude {
    pub use crate::auth::*;
    pub use crate::crud::{CrudService, FieldValue};
    pub use crate::db::{init as db_init, pool as db_pool};
    pub use crate::response::{ApiResponse, ErrorCode};
    pub use crate::validator::validate;
}
