#[derive(Debug, thiserror::Error)]
pub enum DiError {
    #[error(
        "`{provider}` requires `{dep_type}` for field `{field}`, but nothing is registered for \
         that type yet — check `#[module(providers = [...])]` declaration order, or that the \
         providing module is imported. (If this is a circular dependency between two \
         injectables, break the cycle.)"
    )]
    MissingDependency {
        provider: &'static str,
        field: &'static str,
        dep_type: &'static str,
    },
}

impl DiError {
    pub fn missing(provider: &'static str, field: &'static str, dep_type: &'static str) -> Self {
        Self::MissingDependency {
            provider,
            field,
            dep_type,
        }
    }
}
