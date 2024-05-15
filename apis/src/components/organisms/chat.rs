use crate::providers::{
    chat::Chat, game_state::GameStateSignal, navigation_controller::NavigationControllerSignal,
    AuthContext,
};
use leptos::*;
use leptos_use::{use_mutation_observer_with_options, UseMutationObserverOptions};
use shared_types::{ChatDestination, ChatMessage, SimpleDestination};
use uuid::Uuid;

#[component]
pub fn Message(message: ChatMessage) -> impl IntoView {
    let formatted_timestamp = message
        .timestamp
        .unwrap()
        .format("%Y-%m-%d %H:%M")
        .to_string();
    view! {
        <div class="flex items-center mb-1 w-full">
            <div class="px-2 w-full">
                <div class="text-sm select-text">{message.username} at {formatted_timestamp}</div>
                <div class="text-sm break-words select-text max-w-fit">{message.message}</div>
            </div>
        </div>
    }
}

#[component]
pub fn ChatInput(destination: ChatDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let destination = store_value(destination);
    let message_signal = RwSignal::new(String::new());
    let input = move |evt| message_signal.update(|v| *v = event_target_value(&evt));
    let send = move || {
        let message = message_signal();
        if !message.is_empty() {
            chat.send(&message, destination());
            message_signal.set(String::new());
        };
    };
    let placeholder = move || match destination() {
        ChatDestination::GamePlayers(_, _, _) => "Chat with opponent",
        ChatDestination::GameSpectators(_, _, _) => "Chat with spectators",
        _ => "Chat",
    };
    view! {
        <input
            type="text"
            class="box-border px-4 py-2 w-full h-auto rounded-lg resize-none bg-odd-light dark:bg-odd-dark focus:outline-none shrink-0"
            prop:value=message_signal
            attr:placeholder=placeholder
            on:input=input
            on:keydown=move |evt| {
                if evt.key() == "Enter" {
                    evt.prevent_default();
                    send();
                }
            }

            attr:maxlength="1000"
        />
    }
}

#[component]
pub fn ChatWindow(
    destination: SimpleDestination,
    #[prop(optional)] correspondant_id: Option<Uuid>,
    #[prop(optional)] correspondant_username: String,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let game_state = expect_context::<GameStateSignal>();
    let uid = if let Some(Ok(Some(account))) = untrack(auth_context.user) {
        Some(account.user.uid)
    } else {
        None
    };
    let white_id = move || {
        game_state
            .signal
            .get_untracked()
            .white_id
            .expect("Game has white player")
    };
    let black_id = move || {
        game_state
            .signal
            .get_untracked()
            .black_id
            .expect("Game has black player")
    };

    let navi = expect_context::<NavigationControllerSignal>();
    let game = store_value((navi.signal)().nanoid.unwrap_or_default());
    let correspondant_id = store_value(if let Some(v) = correspondant_id {
        v
    } else {
        Uuid::new_v4()
    });
    let correspondant_username = store_value(correspondant_username);
    let div = create_node_ref::<html::Div>();
    let _ = use_mutation_observer_with_options(
        div,
        move |mutations, _| {
            if let Some(_mutation) = mutations.first() {
                let div = div.get_untracked().expect("div to be loaded");
                div.set_scroll_top(div.scroll_height())
            }
        },
        UseMutationObserverOptions::default()
            .child_list(true)
            .attributes(true),
    );

    let actual_destination = move || match destination {
        SimpleDestination::Game => {
            if game_state.signal.get_untracked().uid_is_player(uid) {
                ChatDestination::GamePlayers(game(), white_id(), black_id())
            } else {
                ChatDestination::GameSpectators(game(), white_id(), black_id())
            }
        }
        SimpleDestination::User => {
            ChatDestination::User((correspondant_id(), correspondant_username()))
        }
        SimpleDestination::Tournament => todo!(),
    };
    let cloned_fn = actual_destination.clone();
    let messages = move || match actual_destination() {
        ChatDestination::TournamentLobby(tournament) => (chat.tournament_lobby_messages)()
            .get(&tournament)
            .cloned()
            .unwrap_or_default(),
        ChatDestination::GamePlayers(game, ..) => (chat.games_private_messages)()
            .get(&game)
            .cloned()
            .unwrap_or_default(),

        ChatDestination::GameSpectators(game, ..) => (chat.games_public_messages)()
            .get(&game)
            .cloned()
            .unwrap_or_default(),

        ChatDestination::User((correspondant_id, _username)) => (chat.users_messages)()
            .get(&correspondant_id)
            .cloned()
            .unwrap_or_default(),
    };
    view! {
        <div class="flex flex-col h-full">
            <div ref=div class="overflow-y-auto h-full">
                <For each=messages key=|message| message.timestamp let:message>
                    <Message message=message/>
                </For>
            </div>
            <ChatInput destination=cloned_fn()/>
        </div>
    }
}
