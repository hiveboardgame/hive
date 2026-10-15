use crate::i18n::{t_string, I18nKeys, Locale};
use leptos_i18n::I18nContext;

/// Store the failure, then translate when the UI renders it so locale changes remain reactive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushError {
    EnableFirst,
    TestFailed,
    Unavailable,
    ServiceWorkersUnsupported,
    RegistrationLookupFailed,
    ServiceWorkerMissing,
    PermissionBlocked,
    PermissionRequestFailed,
    PermissionNotGranted,
    ServerKeyMissing,
    PushManagerFailed,
    ServerKeyInvalid,
    SubscribeFailed,
    SubscriptionInvalid,
    RegistrationFailed,
    UnsubscribeFailed,
    SubscriptionNotReleased,
}

impl PushError {
    pub fn message(self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::EnableFirst => {
                t_string!(i18n, notifications.push_errors.enable_first).to_string()
            }
            Self::TestFailed => t_string!(i18n, notifications.push_errors.test_failed).to_string(),
            Self::Unavailable => t_string!(i18n, notifications.push_errors.unavailable).to_string(),
            Self::ServiceWorkersUnsupported => {
                t_string!(i18n, notifications.push_errors.service_workers_unsupported).to_string()
            }
            Self::RegistrationLookupFailed => {
                t_string!(i18n, notifications.push_errors.registration_lookup_failed).to_string()
            }
            Self::ServiceWorkerMissing => {
                t_string!(i18n, notifications.push_errors.service_worker_missing).to_string()
            }
            Self::PermissionBlocked => {
                t_string!(i18n, notifications.push_errors.permission_blocked).to_string()
            }
            Self::PermissionRequestFailed => {
                t_string!(i18n, notifications.push_errors.permission_request_failed).to_string()
            }
            Self::PermissionNotGranted => {
                t_string!(i18n, notifications.push_errors.permission_not_granted).to_string()
            }
            Self::ServerKeyMissing => {
                t_string!(i18n, notifications.push_errors.server_key_missing).to_string()
            }
            Self::PushManagerFailed => {
                t_string!(i18n, notifications.push_errors.push_manager_failed).to_string()
            }
            Self::ServerKeyInvalid => {
                t_string!(i18n, notifications.push_errors.server_key_invalid).to_string()
            }
            Self::SubscribeFailed => {
                t_string!(i18n, notifications.push_errors.subscribe_failed).to_string()
            }
            Self::SubscriptionInvalid => {
                t_string!(i18n, notifications.push_errors.subscription_invalid).to_string()
            }
            Self::RegistrationFailed => {
                t_string!(i18n, notifications.push_errors.registration_failed).to_string()
            }
            Self::UnsubscribeFailed => {
                t_string!(i18n, notifications.push_errors.unsubscribe_failed).to_string()
            }
            Self::SubscriptionNotReleased => {
                t_string!(i18n, notifications.push_errors.subscription_not_released).to_string()
            }
        }
    }
}
