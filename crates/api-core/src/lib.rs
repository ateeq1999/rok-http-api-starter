pub mod crud;
pub mod health;
pub mod migrations;
pub mod response;
pub mod validator;

pub mod prelude {
    pub use crate::crud::{CrudService, FieldValue};
    pub use crate::health;
    pub use crate::response::{ApiResponse, ErrorCode};
    pub use crate::validator::validate;
}
