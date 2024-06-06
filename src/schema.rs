// @generated automatically by Diesel CLI.

diesel::table! {
    balances (user_id) {
        user_id -> Integer,
        balance -> Float,
    }
}

diesel::table! {
    payments (id) {
        id -> Integer,
        user_id -> Integer,
        amount -> Float,
        status -> Text,
        stripe_payment_id -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
        email -> Text,
    }
}

diesel::joinable!(balances -> users (user_id));
diesel::joinable!(payments -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    balances,
    payments,
    users,
);