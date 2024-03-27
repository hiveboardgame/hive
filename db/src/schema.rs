// @generated automatically by Diesel CLI.

diesel::table! {
    challenges (id) {
        id -> Uuid,
        nanoid -> Text,
        challenger_id -> Uuid,
        game_type -> Text,
        rated -> Bool,
        tournament_queen_rule -> Bool,
        color_choice -> Text,
        created_at -> Timestamptz,
        opponent_id -> Nullable<Uuid>,
        visibility -> Text,
        time_mode -> Text,
        time_base -> Nullable<Int4>,
        time_increment -> Nullable<Int4>,
    }
}

diesel::table! {
    games (id) {
        id -> Uuid,
        nanoid -> Text,
        current_player_id -> Uuid,
        black_id -> Uuid,
        finished -> Bool,
        game_status -> Text,
        game_type -> Text,
        history -> Text,
        game_control_history -> Text,
        rated -> Bool,
        tournament_queen_rule -> Bool,
        turn -> Int4,
        white_id -> Uuid,
        white_rating -> Nullable<Float8>,
        black_rating -> Nullable<Float8>,
        white_rating_change -> Nullable<Float8>,
        black_rating_change -> Nullable<Float8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        time_mode -> Text,
        time_base -> Nullable<Int4>,
        time_increment -> Nullable<Int4>,
        last_interaction -> Nullable<Timestamptz>,
        black_time_left -> Nullable<Int8>,
        white_time_left -> Nullable<Int8>,
        speed -> Text,
        hashes -> Array<Nullable<Int8>>,
        conclusion -> Text,
    }
}

diesel::table! {
    games_users (game_id, user_id) {
        game_id -> Uuid,
        user_id -> Uuid,
    }
}

diesel::table! {
    ratings (id) {
        id -> Int4,
        user_uid -> Uuid,
        played -> Int8,
        won -> Int8,
        lost -> Int8,
        draw -> Int8,
        rating -> Float8,
        deviation -> Float8,
        volatility -> Float8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        speed -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        password -> Text,
        email -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        normalized_username -> Text,
    }
}

diesel::joinable!(games_users -> games (game_id));
diesel::joinable!(games_users -> users (user_id));
diesel::joinable!(ratings -> users (user_uid));

diesel::allow_tables_to_appear_in_same_query!(
    challenges,
    games,
    games_users,
    ratings,
    users,
);
