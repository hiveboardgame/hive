use crate::{
    common::GameActionResponse,
    providers::{
        chat::Chat,
        games::GamesSignal,
        AlertType,
        AlertsContext,
        AuthContext,
        AuthIdentity,
        NotificationContext,
        SoundType,
        Sounds,
        TournamentPairingNotice,
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
use shared_types::GameStart;
#[cfg(not(feature = "ssr"))]
use shared_types::TimeMode;
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

pub fn handle_new_game(game_response: GameResponse, pairing_label: Option<String>) {
    let Some(recipient) = expect_context::<AuthContext>()
        .identity
        .get_untracked()
        .and_then(AuthIdentity::user_id)
    else {
        return;
    };
    let opponent = if recipient == game_response.white_player.uid {
        &game_response.black_player
    } else if recipient == game_response.black_player.uid {
        &game_response.white_player
    } else {
        return;
    };
    if let Some(tournament) = &game_response.tournament {
        // TODO: i18n once copy is approved.
        let label = pairing_label.unwrap_or_else(|| String::from("Tournament game"));
        let notice = TournamentPairingNotice {
            recipient,
            tournament_id: tournament.tournament_id.clone(),
            tournament_name: tournament.name.clone(),
            game_id: game_response.game_id.clone(),
            // TODO: i18n once copy is approved.
            description: format!("{label} · vs {}", opponent.username),
        };
        expect_context::<NotificationContext>()
            .tournament_pairings
            .update(|pairings| {
                pairings.insert(notice.game_id.clone(), notice);
            });
    }
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
        if game.finished
            || game.time_mode == TimeMode::Correspondence
            || (game.game_start == GameStart::Ready && !ready_started)
        {
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
            (tournament.format == Format::Arena)
                .then(|| format!("/tournament/{}", tournament.tournament_id.0))
        });
        let background_challenge = game.tournament.is_none()
            && (game.time_mode == TimeMode::RealTime
                || (game.time_mode == TimeMode::Untimed
                    && (game.white_player.bot || game.black_player.bot)));
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
                        background_challenge,
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
    background_challenge: bool,
) -> bool {
    !pathname.starts_with("/analysis")
        && pathname != game_path
        && (background_challenge
            || (visible && focused)
            || arena_path.is_some_and(|arena_path| pathname == arena_path))
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
            false,
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/tournaments",
            game_path,
            Some(arena_path),
            false,
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/tournament/another-arena",
            game_path,
            Some(arena_path),
            false,
        ));
        assert!(actionable_game_should_navigate(
            false,
            false,
            arena_path,
            game_path,
            Some(arena_path),
            false,
        ));
        assert!(!actionable_game_should_navigate(
            true,
            true,
            "/analysis/position",
            game_path,
            Some(arena_path),
            false,
        ));
        assert!(!actionable_game_should_navigate(
            true,
            true,
            game_path,
            game_path,
            Some(arena_path),
            false,
        ));
    }

    #[test]
    fn ordinary_realtime_challenge_opens_in_background_but_respects_analysis_and_current_game() {
        assert!(actionable_game_should_navigate(
            false,
            false,
            "/",
            "/game/new",
            None,
            true
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/analysis/position",
            "/game/new",
            None,
            true
        ));
        assert!(!actionable_game_should_navigate(
            false,
            false,
            "/game/new",
            "/game/new",
            None,
            true
        ));
    }
}
