pub mod config;
pub mod context;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod plugin;
pub mod primitives;
pub mod services;
pub mod session;
pub mod validators;

pub mod prelude {
    pub use crate::config::{AuthConfig, OAuthProviderConfig};
    pub use crate::context::{AuthContext, MailSender, PermissionFinder, UserFinder, UserRecord};
    pub use crate::error::AuthError;
    pub use crate::extractors::{AdminOnly, AuthUser};
    pub use crate::middleware::{AuthStrategy, JwtAuthLayer};
    pub use crate::plugin::AuthPlugin;
    pub use crate::primitives::*;
    pub use crate::session::Session;
    pub use crate::validators::ValidatedJson;
}
