mod actions;
mod catalog;
mod layout;
mod routes;
mod sidebar;
mod thread;

pub use layout::MessagesLayout;
pub use routes::{MessagesDmThread, MessagesGameThread, MessagesTournamentThread};
pub use thread::{MessagesGlobalThread, MessagesIndex};

use shared_types::{GameId, GameThread, TournamentId};

// Messages hub: /message routes are the source of truth for the open thread.

const MESSAGE_ROOT_PATH: &str = "/message";
const MESSAGE_GLOBAL_PATH: &str = "/message/global";
const MESSAGES_PRIMARY_HEADER_CLASS: &str = "flex min-h-11 items-center justify-between gap-3 border-b border-black/10 bg-light px-3 py-2.5 dark:border-white/10 dark:bg-surface-muted xs:px-4 xs:py-3";

fn normalized_message_path(path: &str) -> &str {
    match path.trim_end_matches('/') {
        "" => MESSAGE_ROOT_PATH,
        path => path,
    }
}

fn message_path_is(path: &str, href: &str) -> bool {
    normalized_message_path(path) == href
}

fn message_dm_href(username: &str) -> String {
    format!("/message/dm/{username}")
}

fn message_tournament_href(tournament_id: &TournamentId) -> String {
    format!("/message/tournament/{}", tournament_id.0)
}

fn message_game_href(game_id: &GameId, thread: GameThread) -> String {
    format!("/message/game/{}/{}", game_id.0, thread.slug())
}

fn message_path_matches_game(
    path: &str,
    game_id: &GameId,
    thread: GameThread,
    finished: bool,
) -> bool {
    if finished {
        message_path_is(path, &message_game_href(game_id, GameThread::Players))
            || message_path_is(path, &message_game_href(game_id, GameThread::Spectators))
    } else {
        message_path_is(path, &message_game_href(game_id, thread))
    }
}
