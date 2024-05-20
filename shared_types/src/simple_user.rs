use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SimpleUser {
    pub user_id: Uuid,
    pub username: String,
    pub authed: bool,
    pub admin: bool,
}
