use crate::i18n::{td_string, I18nKeys, Locale};
use leptos_i18n::I18nContext;
use serde::{Deserialize, Serialize};

/// Stable error codes travel through string-only server-function and alert envelopes.
/// Keep English presentation text out of this contract; translate only when rendering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiError {
    RequestUnavailable,
    ChatRetryConflict,
    ChatMuted,
    Unauthorized,
    ChatAccessDenied,
    ChatSubscriptionLimited,
    ChatSendLimited,
    TournamentNameTaken,
    BlockMuteFailed,
    RequestOriginMissing,
    RequestUrlInvalid,
    RequestOriginMismatch,
    RequestTokenMissing,
    RequestUnverified,
    DiscordStartFailed,
    DiscordResponseInvalid,
    DiscordUrlInvalid,
    DiscordStateInvalid,
    DiscordSessionRequired,
    DiscordUnavailable,
    DiscordUnverified,
    DiscordFailed,
    RatingAboveBand { rating: String, limit: String },
    RatingBelowBand { rating: String, limit: String },
    RealtimeGamesPaused,
}

impl UiError {
    pub fn encode(&self) -> String {
        format!("hive_error:{}", serde_json::json!(self))
    }

    fn decode(message: &str) -> Option<Self> {
        serde_json::from_str(message.strip_prefix("hive_error:")?).ok()
    }

    pub fn message(&self, locale: Locale) -> String {
        match self {
            Self::RequestUnavailable => {
                td_string!(locale, messages.errors.request_unavailable).to_string()
            }
            Self::ChatMuted => td_string!(locale, messages.chat.muted_notice).to_string(),
            Self::ChatRetryConflict => {
                td_string!(locale, messages.errors.chat_retry_conflict).to_string()
            }
            Self::Unauthorized => td_string!(locale, messages.errors.unauthorized).to_string(),
            Self::ChatAccessDenied => {
                td_string!(locale, messages.errors.chat_access_denied).to_string()
            }
            Self::ChatSubscriptionLimited => {
                td_string!(locale, messages.errors.chat_subscription_limited).to_string()
            }
            Self::ChatSendLimited => {
                td_string!(locale, messages.errors.chat_send_limited).to_string()
            }
            Self::TournamentNameTaken => {
                td_string!(locale, messages.errors.tournament_name_taken).to_string()
            }
            Self::BlockMuteFailed => {
                td_string!(locale, messages.errors.block_mute_failed).to_string()
            }
            Self::RequestOriginMissing => {
                td_string!(locale, messages.errors.request_origin_missing).to_string()
            }
            Self::RequestUrlInvalid => {
                td_string!(locale, messages.errors.request_url_invalid).to_string()
            }
            Self::RequestOriginMismatch => {
                td_string!(locale, messages.errors.request_origin_mismatch).to_string()
            }
            Self::RequestTokenMissing => {
                td_string!(locale, messages.errors.request_token_missing).to_string()
            }
            Self::RequestUnverified => {
                td_string!(locale, messages.errors.request_unverified).to_string()
            }
            Self::DiscordStartFailed => {
                td_string!(locale, messages.errors.discord_start_failed).to_string()
            }
            Self::DiscordResponseInvalid => {
                td_string!(locale, messages.errors.discord_response_invalid).to_string()
            }
            Self::DiscordUrlInvalid => {
                td_string!(locale, messages.errors.discord_url_invalid).to_string()
            }
            Self::DiscordStateInvalid => {
                td_string!(locale, messages.errors.discord_state_invalid).to_string()
            }
            Self::DiscordSessionRequired => {
                td_string!(locale, messages.errors.discord_session_required).to_string()
            }
            Self::DiscordUnavailable => {
                td_string!(locale, messages.errors.discord_unavailable).to_string()
            }
            Self::DiscordUnverified => {
                td_string!(locale, messages.errors.discord_unverified).to_string()
            }
            Self::DiscordFailed => td_string!(locale, messages.errors.discord_failed).to_string(),
            Self::RatingAboveBand { rating, limit } => td_string!(
                locale,
                messages.errors.rating_above_band,
                rating = rating,
                limit = limit
            )
            .to_string(),
            Self::RatingBelowBand { rating, limit } => td_string!(
                locale,
                messages.errors.rating_below_band,
                rating = rating,
                limit = limit
            )
            .to_string(),
            Self::RealtimeGamesPaused => {
                td_string!(locale, messages.errors.realtime_games_paused).to_string()
            }
        }
    }
}

pub fn localized_error_message(i18n: I18nContext<Locale, I18nKeys>, message: &str) -> String {
    localized_error_message_for_locale(i18n.get_locale(), message)
}

pub fn localized_error_message_for_locale(locale: Locale, message: &str) -> String {
    let reason = [
        "error running server function: ",
        "error reaching server to call server function: ",
        "error running middleware: ",
    ]
    .into_iter()
    .find_map(|prefix| message.strip_prefix(prefix))
    .unwrap_or(message);
    UiError::decode(reason)
        .map(|error| error.message(locale))
        .unwrap_or_else(|| message.to_string())
}

#[cfg(test)]
mod tests {
    use super::{localized_error_message_for_locale, UiError};
    use crate::i18n::Locale;
    use leptos::prelude::ServerFnError;
    use leptos_i18n::Locale as _;

    #[test]
    fn error_codes_and_parameters_survive_server_function_envelopes() {
        let errors = [
            UiError::DiscordStateInvalid,
            UiError::RatingAboveBand {
                rating: "1500.5".into(),
                limit: "1400".into(),
            },
        ];
        for error in errors {
            let encoded = error.encode();
            assert_eq!(UiError::decode(&encoded), Some(error.clone()));
            let wrapped = ServerFnError::new(&encoded).to_string();
            for locale in Locale::get_all() {
                assert_eq!(
                    localized_error_message_for_locale(*locale, &wrapped),
                    error.message(*locale)
                );
            }
        }
    }

    #[test]
    fn unrecognized_errors_remain_readable() {
        for message in ["upstream request failed", "hive_error:unknown_future_code"] {
            assert_eq!(
                localized_error_message_for_locale(Locale::default(), message),
                message
            );
        }
    }
}
