use serde::{Deserialize, Serialize};

/// Platform-agnostic push payload. The FCM / APNs notifier impls translate
/// this into their respective JSON shapes.
///
/// `link` is the canonical deep-link URL the tap should open (e.g.
/// `https://hivegame.com/game/<nanoid>`). The mobile `DeepLinkListener`
/// already handles routing on tap; the OS hands us the URL via the
/// notification payload's data section.
///
/// `event_type` lets client-side filtering distinguish channels (your_turn,
/// challenge, game_ended, …) for future per-event-type display preferences
/// without us having to add a new top-level field per event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Push {
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub event_type: String,
}
