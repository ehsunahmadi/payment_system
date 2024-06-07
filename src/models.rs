use diesel::prelude::*;

#[derive(serde::Serialize, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub balance: String,
}

#[derive(serde::Deserialize, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub username: String,
    pub email: String,
}

#[derive(serde::Serialize, Selectable, Queryable)]
#[diesel(table_name = crate::schema::payments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Payment {
    pub id: i32,
    pub user_id: i32,
    pub amount: String,
    pub status: String,
    pub session_id: String,
    pub created_at: String,
}

#[derive(serde::Deserialize, Insertable)]
#[diesel(table_name = crate::schema::payments)]
pub struct NewPayment {
    pub user_id: i32,
    pub amount: String,
    pub session_id: String,
}
