// @generated automatically by Diesel CLI.

diesel::table! {
    payments (id) {
        id -> Integer,
        user_id -> Integer,
        amount -> Text,
        status -> Text,
        session_id -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
        email -> Text,
        balance -> Text,
    }
}

diesel::joinable!(payments -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    payments,
    users,
);
