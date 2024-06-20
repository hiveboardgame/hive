use crate::common::MoveConfirm;
use leptos::*;
use shared_types::GameSpeed;

#[server]
pub async fn toggle_confirm_mode(
    move_confirm: MoveConfirm,
    game_speed: GameSpeed,
) -> Result<(GameSpeed, MoveConfirm), ServerFnError> {
    use actix_web::http::header::{HeaderMap, HeaderValue, SET_COOKIE};
    use chrono::Duration;
    use leptos_actix::{ResponseOptions, ResponseParts};

    let response = expect_context::<ResponseOptions>();
    let mut response_parts = ResponseParts::default();
    let mut headers = HeaderMap::new();
    let max_age = Duration::days(365)
        .to_std()
        .expect("Not a negative duration")
        .as_secs();

    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{game_speed}_confirm_mode={move_confirm}; Max-Age={max_age}; Path=/",
        ))
        .expect("to create header value"),
    );
    response_parts.headers = headers;

    response.overwrite(response_parts);
    Ok((game_speed, move_confirm))
}
