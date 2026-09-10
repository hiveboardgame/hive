use crate::{
    common::GameActionResponse,
    providers::{
        chat::Chat,
        games::GamesSignal,
        AlertType,
        AlertsContext,
        AuthContext,
        AuthIdentity,
        SoundType,
        Sounds,
        UpdateNotifier,
    },
    responses::GameResponse,
};
use hive_lib::GameControl;
#[cfg(not(feature = "ssr"))]
use leptos::leptos_dom::helpers::queue_microtask;
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
#[cfg(not(feature = "ssr"))]
use shared_types::tournament::Format;
use shared_types::{GameStart, TimeMode};
#[cfg(not(feature = "ssr"))]
use web_sys::VisibilityState;

pub fn handle_control(game_control: GameControl, gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let game_updater = expect_context::<UpdateNotifier>();
    game_updater.game_response.set(Some(gar.clone()));
    let aborted = matches!(game_control, GameControl::Abort(_));
    match game_control {
        GameControl::Abort(_) => {
            games.own_games_remove(&gar.game.game_id);
            let chat = expect_context::<Chat>();
            chat.clear_game_thread(&gar.game.game_id);
            chat.request_catalog_refresh();
            let alerts = expect_context::<AlertsContext>();
            alerts.last_alert.update(|v| {
                *v = Some(AlertType::Warn(format!(
                    "{} aborted the game",
                    gar.username
                )));
            });

            let location = use_location();
            let current_path = location.pathname.get_untracked();
            let game_path = format!("/game/{}", gar.game.game_id);

            if current_path.starts_with(&game_path) {
                let navigate = use_navigate();
                navigate("/", Default::default());
            }
        }
        GameControl::DrawAccept(_) => {
            games.own_games_remove(&gar.game.game_id);
        }
        GameControl::Resign(_) => {
            games.own_games_remove(&gar.game.game_id);
        }
        GameControl::TakebackAccept(_) => {
            games.own_games_add(gar.game.to_owned());
        }
        GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
            games.own_games_add(gar.game.to_owned());
        }
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {}
    }
    if gar.game.finished && !aborted {
        let chat = expect_context::<Chat>();
        chat.request_catalog_refresh();
    }
}

pub fn handle_new_game(game_response: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    if game_response.game_start != GameStart::Ready {
        let sounds = expect_context::<Sounds>();
        games.own_games_add(game_response.to_owned());
        sounds.play_sound(SoundType::NewGame);
        navigate_to_actionable_game(&game_response, false);
    }
}

pub fn navigate_to_actionable_game(game: &GameResponse, ready_started: bool) {
    #[cfg(not(feature = "ssr"))]
    {
        if game.finished || (game.game_start == GameStart::Ready && !ready_started) {
            return;
        }
        let auth = expect_context::<AuthContext>();
        let Some(expected_identity) = auth.identity.get_untracked() else {
            return;
        };
        let Some(user_id) = expected_identity.user_id() else {
            return;
        };
        if user_id != game.white_player.uid && user_id != game.black_player.uid {
            return;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(expected_pathname) = window.location().pathname() else {
            return;
        };
        let Some(owner) = Owner::current().map(|owner| owner.downgrade()) else {
            return;
        };
        let game_path = format!("/game/{}", game.game_id);
        let arena_path = game.tournament.as_ref().and_then(|tournament| {
            (tournament.format() == Format::Arena)
                .then(|| format!("/tournament/{}", tournament.tournament_id.0))
        });
        let navigate = use_navigate();

        queue_microtask(move || {
            let Some(owner) = owner.upgrade() else {
                return;
            };
            owner.with(|| {
                if auth.identity.get_untracked() != Some(expected_identity) {
                    return;
                }
                let Some(window) = web_sys::window() else {
                    return;
                };
                let Some(document) = window.document() else {
                    return;
                };
                let Ok(pathname) = window.location().pathname() else {
                    return;
                };
                if pathname != expected_pathname
                    || !actionable_game_should_navigate(
                        document.visibility_state() == VisibilityState::Visible,
                        document.has_focus().unwrap_or(false),
                        &pathname,
                        &game_path,
                        arena_path.as_deref(),
                    )
                {
                    return;
                }
                navigate(&game_path, Default::default());
            });
        });
    }
    #[cfg(feature = "ssr")]
    let _ = (game, ready_started);
}

#[cfg(any(not(feature = "ssr"), test))]
fn actionable_game_should_navigate(
    visible: bool,
    focused: bool,
    pathname: &str,
    game_path: &str,
    arena_path: Option<&str>,
) -> bool {
    !pathname.starts_with("/analysis")
        && pathname != game_path
        && ((visible && focused) || arena_path.is_some_and(|arena_path| pathname == arena_path))
}

pub fn handle_untimed_bot_navigation(game: &GameResponse) {
    let Some(user_id) = expect_context::<AuthContext>()
        .identity
        .get_untracked()
        .and_then(AuthIdentity::user_id)
    else {
        return;
    };
    let location = use_location();
    if !untimed_bot_should_navigate(
        game.time_mode,
        game.white_player.bot || game.black_player.bot,
        user_id == game.white_player.uid || user_id == game.black_player.uid,
        &location.pathname.get_untracked(),
    ) {
        return;
    }
    use_navigate()(&format!("/game/{}", game.game_id), Default::default());
}

fn untimed_bot_should_navigate(
    time_mode: TimeMode,
    has_bot: bool,
    user_is_player: bool,
    pathname: &str,
) -> bool {
    time_mode == TimeMode::Untimed
        && has_bot
        && user_is_player
        && !pathname.starts_with("/analysis")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_assignment_navigation_is_scoped_to_the_matching_arena_in_background() {
        let game_path = "/game/assigned";
        let arena_path = "/tournament/arena-one";

        assert!(actionable_game_should_navigate(
            true,
            true,
            "/tournaments",
            game_path,
            None,
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/tournaments",
            game_path,
            Some(arena_path),
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/tournament/another-arena",
            game_path,
            Some(arena_path),
        ));
        assert!(actionable_game_should_navigate(
            false,
            false,
            arena_path,
            game_path,
            Some(arena_path),
        ));
        assert!(!actionable_game_should_navigate(
            true,
            true,
            "/analysis/position",
            game_path,
            Some(arena_path),
        ));
        assert!(!actionable_game_should_navigate(
            true,
            true,
            game_path,
            game_path,
            Some(arena_path),
        ));
    }

    #[test]
    fn legacy_untimed_bot_navigation_remains_narrow_and_analysis_safe() {
        assert!(untimed_bot_should_navigate(
            TimeMode::Untimed,
            true,
            true,
            "/"
        ));
        assert!(!untimed_bot_should_navigate(
            TimeMode::RealTime,
            true,
            true,
            "/"
        ));
        assert!(!untimed_bot_should_navigate(
            TimeMode::Untimed,
            false,
            true,
            "/"
        ));
        assert!(!untimed_bot_should_navigate(
            TimeMode::Untimed,
            true,
            false,
            "/"
        ));
        assert!(!untimed_bot_should_navigate(
            TimeMode::Untimed,
            true,
            true,
            "/analysis/game"
        ));
    }
}
