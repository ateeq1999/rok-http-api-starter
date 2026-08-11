mod container;
mod error;
mod extractor;

use std::sync::Arc;

pub use container::{Binder, Container, ContainerBuilder};
pub use di_macros::{injectable, module};
pub use error::DiError;
pub use extractor::{DiRejection, Injected};

/// Implemented by `#[injectable]` for structs whose every field is `#[inject]`. Not meant to be
/// implemented by hand — see the `#[injectable]` macro.
pub trait Injectable: Sized {
    fn construct(container: &ContainerBuilder) -> Result<Arc<Self>, DiError>;
}

/// Implemented by `#[module(...)]`. Not meant to be implemented by hand.
pub trait Module {
    fn register(builder: &mut ContainerBuilder) -> Result<(), DiError>;
}

pub mod prelude {
    pub use crate::{injectable, module, Container, ContainerBuilder, DiError, Injectable, Injected, Module};
}
