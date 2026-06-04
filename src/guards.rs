use rok_auth::axum::guard::RoleMarker;

pub struct Admin;
impl RoleMarker for Admin {
    const ROLE: &'static str = "admin";
}

#[allow(dead_code)]
pub struct User;
impl RoleMarker for User {
    const ROLE: &'static str = "user";
}
