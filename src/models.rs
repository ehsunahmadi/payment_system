use diesel::prelude::*;

#[derive(serde::Serialize, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
}

#[derive(serde::Deserialize, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub username: String,
    pub email: String,
}

#[derive(serde::Serialize, Queryable, Selectable)]
#[diesel(table_name = crate::schema::balances)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Balances {
    pub user_id: i32,
    pub balance: i32,
}

#[derive(serde::Serialize, Selectable, Queryable)]
#[diesel(table_name = crate::schema::payments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Payment {
    pub id: i32,
    pub user_id: i32,
    pub amount: i32,
    pub status: String,
    pub stripe_payment_id: String,
    pub created_at: String,
}

#[derive(serde::Deserialize, Insertable)]
#[diesel(table_name = crate::schema::payments)]
pub struct NewPayment {
    pub user_id: i32,
    pub amount: i32,
}
