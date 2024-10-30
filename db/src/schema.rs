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
        band_upper -> Nullable<Int4>,
        band_lower -> Nullable<Int4>,
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
        tournament_id -> Nullable<Uuid>,
        tournament_game_result -> Text,
        game_start -> Text,
        move_times -> Array<Nullable<Int8>>,
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
    schedules (id) {
        id -> Uuid,
        game_id -> Uuid,
        tournament_id -> Uuid,
        proposer_id -> Uuid,
        opponent_id -> Uuid,
        start_t -> Timestamptz,
        agreed -> Bool,
    }
}

diesel::table! {
    tournament_series (id) {
        id -> Uuid,
        nanoid -> Text,
        name -> Text,
        description -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    tournament_series_organizers (tournament_series_id, organizer_id) {
        tournament_series_id -> Uuid,
        organizer_id -> Uuid,
    }
}

diesel::table! {
    tournaments (id) {
        id -> Uuid,
        nanoid -> Text,
        name -> Text,
        description -> Text,
        scoring -> Text,
        tiebreaker -> Array<Nullable<Text>>,
        seats -> Int4,
        min_seats -> Int4,
        rounds -> Int4,
        invite_only -> Bool,
        mode -> Text,
        time_mode -> Text,
        time_base -> Nullable<Int4>,
        time_increment -> Nullable<Int4>,
        band_upper -> Nullable<Int4>,
        band_lower -> Nullable<Int4>,
        start_mode -> Text,
        starts_at -> Nullable<Timestamptz>,
        ends_at -> Nullable<Timestamptz>,
        started_at -> Nullable<Timestamptz>,
        round_duration -> Nullable<Int4>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        series -> Nullable<Uuid>,
    }
}

diesel::table! {
    tournaments_invitations (tournament_id, invitee_id) {
        tournament_id -> Uuid,
        invitee_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    tournaments_organizers (tournament_id, organizer_id) {
        tournament_id -> Uuid,
        organizer_id -> Uuid,
    }
}

diesel::table! {
    tournaments_users (tournament_id, user_id) {
        tournament_id -> Uuid,
        user_id -> Uuid,
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
        patreon -> Bool,
        admin -> Bool,
        takeback -> Text,
    }
}

diesel::joinable!(games_users -> games (game_id));
diesel::joinable!(games_users -> users (user_id));
diesel::joinable!(ratings -> users (user_uid));
diesel::joinable!(schedules -> games (game_id));
diesel::joinable!(schedules -> tournaments (tournament_id));
diesel::joinable!(tournament_series_organizers -> tournament_series (tournament_series_id));
diesel::joinable!(tournament_series_organizers -> users (organizer_id));
diesel::joinable!(tournaments -> tournament_series (series));
diesel::joinable!(tournaments_invitations -> tournaments (tournament_id));
diesel::joinable!(tournaments_invitations -> users (invitee_id));
diesel::joinable!(tournaments_organizers -> tournaments (tournament_id));
diesel::joinable!(tournaments_organizers -> users (organizer_id));
diesel::joinable!(tournaments_users -> tournaments (tournament_id));
diesel::joinable!(tournaments_users -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    challenges,
    games,
    games_users,
    ratings,
    schedules,
    tournament_series,
    tournament_series_organizers,
    tournaments,
    tournaments_invitations,
    tournaments_organizers,
    tournaments_users,
    users,
);
