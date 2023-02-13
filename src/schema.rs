// @generated automatically by Diesel CLI.

diesel::table! {
    devices (id) {
        id -> Uuid,
        #[sql_name = "type"]
        type_ -> Varchar,
        user_id -> Int4,
        secret -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        email -> Varchar,
        password -> Varchar,
        first_name -> Varchar,
        last_name -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    devices,
    users,
);
