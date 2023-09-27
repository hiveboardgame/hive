// @generated automatically by Diesel CLI.

diesel::table! {
    challenges (id) {
        id -> Uuid,
        challenger_uid -> Text,
        game_type -> Text,
        rated -> Bool,
        public -> Bool,
        tournament_queen_rule -> Bool,
        color_choice -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    games (id) {
        id -> Int4,
        black_uid -> Text,
        game_status -> Text,
        game_type -> Text,
        history -> Text,
        game_control_history -> Text,
        rated -> Bool,
        tournament_queen_rule -> Bool,
        turn -> Int4,
        white_uid -> Text,
        white_rating -> Nullable<Float8>,
        black_rating -> Nullable<Float8>,
        white_rating_change -> Nullable<Float8>,
        black_rating_change -> Nullable<Float8>,
    }
}

diesel::table! {
    games_users (game_id, user_uid) {
        game_id -> Int4,
        user_uid -> Text,
    }
}

diesel::table! {
    ratings (id) {
        id -> Int4,
        user_uid -> Text,
        played -> Int8,
        won -> Int8,
        lost -> Int8,
        draw -> Int8,
        rating -> Float8,
        deviation -> Float8,
        volatility -> Float8,
    }
}

diesel::table! {
    users (uid) {
        uid -> Text,
        username -> Text,
        password -> Text,
        token -> Text,
    }
}

diesel::joinable!(challenges -> users (challenger_uid));
diesel::joinable!(games_users -> games (game_id));
diesel::joinable!(games_users -> users (user_uid));
diesel::joinable!(ratings -> users (user_uid));

diesel::allow_tables_to_appear_in_same_query!(
    challenges,
    games,
    games_users,
    ratings,
    users,
);
