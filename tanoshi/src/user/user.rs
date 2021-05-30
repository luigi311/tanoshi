use async_graphql::SimpleObject;

#[derive(Debug, SimpleObject)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[graphql(skip)]
    pub password: String,
    pub is_admin: bool,
}
