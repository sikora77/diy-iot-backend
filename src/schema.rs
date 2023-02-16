// @generated automatically by Diesel CLI.

diesel::table! {
    devices (id) {
        id -> Uuid,
        #[sql_name = "type"]
        type_ -> Varchar,
        user_id -> Int4,
        internal_name -> Text,
        name -> Text,
        nicknames -> Array<Nullable<Text>>,
        traits -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    lights (light_id) {
        light_id -> Uuid,
        rgb -> Int4,
        brightness -> Int4,
        is_on -> Bool,
        user_id -> Int4,
        secret -> Varchar,
    }
}

diesel::table! {
    traits (id) {
        id -> Int4,
        device_type -> Text,
        #[sql_name = "trait"]
        trait_ -> Text,
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
    lights,
    traits,
    users,
);
