use crate::{
    functions::{accounts::get::get_account, auth::logout::Logout},
    providers::websocket::WebsocketContext,
    responses::AccountResponse,
};
use codee::string::FromToStringCodec;
use leptos::prelude::*;
use leptos_use::{use_broadcast_channel, UseBroadcastChannelReturn};
use std::sync::Arc;
use uuid::Uuid;

const AUTH_SESSION_CHANNEL: &str = "hive-auth-session";

type SessionChangeFn = Arc<dyn Fn() + Send + Sync>;

struct AuthSessionControls {
    websocket: WebsocketContext,
    notify_session_changed: SessionChangeFn,
}

#[derive(Clone, Copy)]
pub(crate) struct AuthSessionActions {
    state: RwSignal<AuthState>,
    account_refresh_generation: RwSignal<u64>,
    controls: StoredValue<AuthSessionControls>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthIdentity {
    Anonymous,
    User(Uuid),
}

impl AuthIdentity {
    pub(crate) fn user_id(self) -> Option<Uuid> {
        match self {
            Self::Anonymous => None,
            Self::User(user_id) => Some(user_id),
        }
    }
}

#[derive(Clone, Debug)]
enum AuthState {
    Loading,
    Anonymous,
    User(AccountResponse),
}

impl AuthState {
    fn identity(&self) -> Option<AuthIdentity> {
        match self {
            Self::Loading => None,
            Self::Anonymous => Some(AuthIdentity::Anonymous),
            Self::User(account) => Some(AuthIdentity::User(account.user.uid)),
        }
    }
}

#[derive(Clone)]
pub struct AuthContext {
    pub logout: ServerAction<Logout>,
    pub user: Signal<Option<AccountResponse>>,
    pub identity: Signal<Option<AuthIdentity>>,
    pub admin: Signal<Option<bool>>,
    account_refresh: Action<bool, Result<Option<AccountResponse>, ServerFnError>>,
    session_actions: AuthSessionActions,
}

impl AuthContext {
    pub fn accept_user(&self, account: AccountResponse) {
        self.session_actions
            .accept_session(AuthState::User(account));
    }

    pub fn accept_anonymous(&self) {
        self.session_actions.accept_session(AuthState::Anonymous);
    }

    pub fn accept_same_user_refresh(&self, account: AccountResponse) {
        self.session_actions.accept_same_user_refresh(account);
    }

    pub fn refresh_account(&self) {
        self.account_refresh.dispatch(false);
    }

    pub(crate) fn session_actions(&self) -> AuthSessionActions {
        self.session_actions
    }
}

impl AuthSessionActions {
    pub(crate) fn accept_anonymous(self) {
        self.accept_session(AuthState::Anonymous);
    }

    pub(crate) fn accept_same_user_refresh(self, account: AccountResponse) {
        apply_same_user_refresh(self.state, self.account_refresh_generation, account);
    }

    fn accept_session(self, next: AuthState) {
        self.controls.with_value(|controls| {
            accept_session(
                self.state,
                self.account_refresh_generation,
                &controls.websocket,
                next,
            );
            (controls.notify_session_changed)();
        });
    }
}

fn bump_refresh_generation(generation: RwSignal<u64>) -> u64 {
    generation.update(|generation| *generation = generation.saturating_add(1));
    generation.get_untracked()
}

fn accept_session(
    state: RwSignal<AuthState>,
    refresh_generation: RwSignal<u64>,
    websocket: &WebsocketContext,
    next: AuthState,
) {
    bump_refresh_generation(refresh_generation);
    state.set(next);
    reconnect_websocket(websocket);
}

fn apply_same_user_refresh(
    state: RwSignal<AuthState>,
    refresh_generation: RwSignal<u64>,
    account: AccountResponse,
) -> bool {
    let same_user = state.with_untracked(
        |state| matches!(state, AuthState::User(current) if current.user.uid == account.user.uid),
    );
    if same_user {
        bump_refresh_generation(refresh_generation);
        state.set(AuthState::User(account));
    }
    same_user
}

fn reconnect_websocket(websocket: &WebsocketContext) {
    websocket.close();
    websocket.open();
}

fn replace_state(state: RwSignal<AuthState>, next: AuthState) -> bool {
    let previous_identity = state.with_untracked(AuthState::identity);
    let next_identity = next.identity();
    state.set(next);
    previous_identity.is_some() && previous_identity != next_identity
}

fn apply_account_refresh(
    state: RwSignal<AuthState>,
    websocket: &WebsocketContext,
    account: Option<AccountResponse>,
    force_reconnect: bool,
) {
    let next = match account {
        Some(account) => AuthState::User(account),
        None => AuthState::Anonymous,
    };
    if replace_state(state, next) || force_reconnect {
        reconnect_websocket(websocket);
    }
}

fn apply_account_refresh_result(
    state: RwSignal<AuthState>,
    refresh_generation: RwSignal<u64>,
    force_reconnect_pending: RwSignal<bool>,
    websocket: &WebsocketContext,
    request_generation: u64,
    expected_identity: Option<AuthIdentity>,
    result: &Result<Option<AccountResponse>, ServerFnError>,
) {
    let current = refresh_generation.get_untracked() == request_generation
        && state.with_untracked(AuthState::identity) == expected_identity;
    if current {
        match result {
            Ok(account) => {
                let force_reconnect = force_reconnect_pending.get_untracked();
                force_reconnect_pending.set(false);
                apply_account_refresh(state, websocket, account.clone(), force_reconnect)
            }
            Err(_) if expected_identity.is_none() => {
                apply_account_refresh(state, websocket, None, false);
            }
            Err(_) => {}
        }
    }
}

pub fn provide_auth() {
    let websocket = expect_context::<WebsocketContext>();
    let UseBroadcastChannelReturn {
        message: session_changed,
        post: post_session_changed,
        ..
    } = use_broadcast_channel::<Uuid, FromToStringCodec>(AUTH_SESSION_CHANNEL);
    let notify_session_changed = Arc::new(move || post_session_changed(&Uuid::new_v4()));
    let wake_resync_epoch = websocket.wake_resync_epoch;
    let session_controls = StoredValue::new(AuthSessionControls {
        websocket,
        notify_session_changed,
    });
    let logout = ServerAction::<Logout>::new();
    let state = RwSignal::new(AuthState::Loading);
    let account_refresh_generation = RwSignal::new(0);
    let force_reconnect_pending = RwSignal::new(false);
    let account_refresh = Action::new(move |force_reconnect: &bool| {
        if *force_reconnect {
            force_reconnect_pending.set(true);
        }
        let request_generation = bump_refresh_generation(account_refresh_generation);
        let expected_identity = state.with_untracked(AuthState::identity);
        async move {
            let result = get_account().await;
            session_controls.with_value(|controls| {
                apply_account_refresh_result(
                    state,
                    account_refresh_generation,
                    force_reconnect_pending,
                    &controls.websocket,
                    request_generation,
                    expected_identity,
                    &result,
                );
            });
            result
        }
    });

    #[cfg(not(feature = "ssr"))]
    account_refresh.dispatch(false);

    let user = Signal::derive(move || {
        state.with(|state| match state {
            AuthState::User(account) => Some(account.clone()),
            AuthState::Loading | AuthState::Anonymous => None,
        })
    });
    let identity = Signal::derive(move || state.with(AuthState::identity));
    let admin = Signal::derive(move || {
        state.with(|state| match state {
            AuthState::Loading => None,
            AuthState::Anonymous => Some(false),
            AuthState::User(account) => Some(account.user.admin),
        })
    });
    let session_actions = AuthSessionActions {
        state,
        account_refresh_generation,
        controls: session_controls,
    };

    let context = AuthContext {
        user,
        identity,
        admin,
        logout,
        account_refresh,
        session_actions,
    };
    provide_context(context);

    Effect::watch(
        move || session_changed.get(),
        move |session_changed, _, _| {
            if session_changed.is_some() {
                account_refresh.dispatch(true);
            }
        },
        false,
    );

    Effect::watch(
        move || wake_resync_epoch.get(),
        move |_, _, _| account_refresh.dispatch(false),
        false,
    );

    Effect::watch(
        logout.version(),
        move |_, _, _| {
            if logout
                .value()
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                session_actions.accept_anonymous();
            }
        },
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::ClientRequest,
        providers::websocket::ConnectionReadyState,
        responses::UserResponse,
    };
    use shared_types::Takeback;
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    fn account(user_id: Uuid, username: &str) -> AccountResponse {
        AccountResponse {
            username: username.to_string(),
            email: format!("{username}@example.com"),
            id: user_id,
            user: UserResponse {
                username: username.to_string(),
                uid: user_id,
                patreon: false,
                bot: false,
                admin: false,
                deleted: false,
                ratings: HashMap::new(),
                takeback: Takeback::Always,
                lang: None,
            },
        }
    }

    fn websocket_with_transition_counts() -> (WebsocketContext, Arc<AtomicUsize>, Arc<AtomicUsize>)
    {
        let opens = Arc::new(AtomicUsize::new(0));
        let closes = Arc::new(AtomicUsize::new(0));
        let open_count = Arc::clone(&opens);
        let close_count = Arc::clone(&closes);
        let websocket = WebsocketContext::new(
            Signal::derive(|| None),
            Arc::new(|_: &ClientRequest| true),
            Signal::derive(|| ConnectionReadyState::Open),
            Arc::new(move || {
                open_count.fetch_add(1, Ordering::Relaxed);
            }),
            Arc::new(move || {
                close_count.fetch_add(1, Ordering::Relaxed);
            }),
            Arc::new(|| {}),
        );
        (websocket, opens, closes)
    }

    #[test]
    fn same_user_refresh_updates_metadata_without_replacing_the_session() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::User(account(user_id, "before")));
        let refresh_generation = RwSignal::new(0);
        let (websocket, opens, closes) = websocket_with_transition_counts();

        apply_account_refresh(state, &websocket, Some(account(user_id, "after")), false);
        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(user_id))
        );
        assert!(matches!(
            state.get_untracked(),
            AuthState::User(account) if account.username == "after"
        ));
        assert!(apply_same_user_refresh(
            state,
            refresh_generation,
            account(user_id, "edited"),
        ));
        assert!(matches!(
            state.get_untracked(),
            AuthState::User(account) if account.username == "edited"
        ));
        assert_eq!(opens.load(Ordering::Relaxed), 0);
        assert_eq!(closes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn initial_refresh_updates_state_without_reconnecting() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::Loading);
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(false);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            None,
            &Ok(Some(account(user_id, "initial"))),
        );

        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(user_id))
        );
        assert_eq!(opens.load(Ordering::Relaxed), 0);
        assert_eq!(closes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn cross_tab_refresh_reconnects_when_loading_resolves() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::Loading);
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(true);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            None,
            &Ok(Some(account(user_id, "switched"))),
        );

        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(user_id))
        );
        assert_eq!(opens.load(Ordering::Relaxed), 1);
        assert_eq!(closes.load(Ordering::Relaxed), 1);
        assert!(!force_reconnect_pending.get_untracked());
    }

    #[test]
    fn cross_tab_refresh_reconnects_once_for_the_same_user() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::User(account(user_id, "before")));
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(true);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);
        let expected_identity = state.with_untracked(AuthState::identity);

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            expected_identity,
            &Ok(Some(account(user_id, "after"))),
        );

        assert!(matches!(
            state.get_untracked(),
            AuthState::User(account) if account.username == "after"
        ));
        assert_eq!(opens.load(Ordering::Relaxed), 1);
        assert_eq!(closes.load(Ordering::Relaxed), 1);
        assert!(!force_reconnect_pending.get_untracked());
    }

    #[test]
    fn stale_forced_refresh_is_consumed_by_the_next_current_success() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::Loading);
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(true);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let stale_request_generation = bump_refresh_generation(refresh_generation);
        let current_request_generation = bump_refresh_generation(refresh_generation);

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            stale_request_generation,
            None,
            &Ok(Some(account(Uuid::new_v4(), "stale"))),
        );

        assert!(matches!(state.get_untracked(), AuthState::Loading));
        assert!(force_reconnect_pending.get_untracked());
        assert_eq!(opens.load(Ordering::Relaxed), 0);
        assert_eq!(closes.load(Ordering::Relaxed), 0);

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            current_request_generation,
            None,
            &Ok(Some(account(user_id, "current"))),
        );

        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(user_id))
        );
        assert!(!force_reconnect_pending.get_untracked());
        assert_eq!(opens.load(Ordering::Relaxed), 1);
        assert_eq!(closes.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn forced_transient_refresh_failure_retains_the_session_and_reconnect_intent() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let state = RwSignal::new(AuthState::User(account(user_id, "retained")));
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(true);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);
        let expected_identity = state.with_untracked(AuthState::identity);
        let failure: Result<Option<AccountResponse>, ServerFnError> =
            Err(ServerFnError::new("temporary failure"));

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            expected_identity,
            &failure,
        );

        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(user_id))
        );
        assert_eq!(opens.load(Ordering::Relaxed), 0);
        assert_eq!(closes.load(Ordering::Relaxed), 0);
        assert!(force_reconnect_pending.get_untracked());
    }

    #[test]
    fn initial_refresh_failure_resolves_as_anonymous() {
        let owner = Owner::new();
        owner.set();
        let state = RwSignal::new(AuthState::Loading);
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(false);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);
        let failure: Result<Option<AccountResponse>, ServerFnError> =
            Err(ServerFnError::new("temporary failure"));

        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            None,
            &failure,
        );

        assert!(matches!(state.get_untracked(), AuthState::Anonymous));
        assert_eq!(opens.load(Ordering::Relaxed), 0);
        assert_eq!(closes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn stale_initial_refresh_cannot_overwrite_an_explicit_login() {
        let owner = Owner::new();
        owner.set();
        let state = RwSignal::new(AuthState::Loading);
        let refresh_generation = RwSignal::new(0);
        let force_reconnect_pending = RwSignal::new(false);
        let (websocket, opens, closes) = websocket_with_transition_counts();
        let request_generation = bump_refresh_generation(refresh_generation);
        let expected_identity = state.with_untracked(AuthState::identity);
        let login_id = Uuid::new_v4();

        accept_session(
            state,
            refresh_generation,
            &websocket,
            AuthState::User(account(login_id, "login")),
        );
        apply_account_refresh_result(
            state,
            refresh_generation,
            force_reconnect_pending,
            &websocket,
            request_generation,
            expected_identity,
            &Ok(None),
        );

        assert_eq!(
            state.with_untracked(AuthState::identity),
            Some(AuthIdentity::User(login_id))
        );
        assert_eq!(opens.load(Ordering::Relaxed), 1);
        assert_eq!(closes.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn mismatched_same_user_refresh_cannot_resurrect_a_session() {
        let owner = Owner::new();
        owner.set();
        let state = RwSignal::new(AuthState::Anonymous);
        let refresh_generation = RwSignal::new(3);

        assert!(!apply_same_user_refresh(
            state,
            refresh_generation,
            account(Uuid::new_v4(), "stale"),
        ));

        assert!(matches!(state.get_untracked(), AuthState::Anonymous));
        assert_eq!(refresh_generation.get_untracked(), 3);
    }
}
