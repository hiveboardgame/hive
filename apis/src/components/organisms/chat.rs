use crate::{
    components::update_from_event::update_from_input,
    providers::{
        chat::Chat, game_state::GameStateSignal, navigation_controller::NavigationControllerSignal,
        AuthContext,
    },
};
use chrono::Local;
use leptos::*;
use leptos_use::{use_mutation_observer_with_options, UseMutationObserverOptions};
use shared_types::{ChatDestination, ChatMessage, SimpleDestination};
use uuid::Uuid;

#[component]
pub fn Message(message: ChatMessage) -> impl IntoView {
    let user_local_time = message
        .timestamp
        .unwrap()
        .with_timezone(&Local)
        .format(" %d/%m/%Y %H:%M")
        .to_string();
    let turn = message.turn.map(|turn| (format!(" on turn {turn}:")));

    view! {
        <div class="flex flex-col items-start mb-1 w-full">
            <div class="flex gap-1 px-2">
                <div class="font-bold">{message.username}</div>
                {user_local_time}
                {turn}
            </div>
            <div class="px-2 break-words max-w-fit">{message.message}</div>
        </div>
    }
}

#[component]
pub fn ChatInput(destination: ChatDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let destination = store_value(destination);
    let send = move || {
        batch(move || {
            let message = chat.typed_message.get();
            if !message.is_empty() {
                chat.send(&message, destination());
                chat.typed_message.set(String::new());
            };
        })
    };
    let placeholder = move || match destination() {
        ChatDestination::GamePlayers(_, _, _) => "Chat with opponent",
        ChatDestination::GameSpectators(_, _, _) => "Chat with spectators",
        _ => "Chat",
    };
    let my_input = NodeRef::<html::Input>::new();
    create_effect(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    view! {
        <input
            ref=my_input
            type="text"
            class="box-border px-4 py-2 w-full h-auto rounded-lg resize-none bg-odd-light dark:bg-odd-dark focus:outline-none shrink-0"
            prop:value=chat.typed_message
            attr:placeholder=placeholder
            on:input=update_from_input(chat.typed_message)
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
    let game_id = store_value(
        navi.game_signal
            .get_untracked()
            .game_id
            .unwrap_or_default(),
    );
    let correspondant_id = store_value(correspondant_id.map_or(Uuid::new_v4(), |id| id));
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
                ChatDestination::GamePlayers(game_id(), white_id(), black_id())
            } else {
                ChatDestination::GameSpectators(game_id(), white_id(), black_id())
            }
        }
        SimpleDestination::User => {
            ChatDestination::User((correspondant_id(), correspondant_username()))
        }
        SimpleDestination::Global => ChatDestination::Global,
        SimpleDestination::Tournament => todo!(),
    };
    let cloned_fn = actual_destination.clone();
    let messages = move || match actual_destination() {
        ChatDestination::TournamentLobby(tournament) => (chat.tournament_lobby_messages)()
            .get(&tournament)
            .cloned()
            .unwrap_or_default(),
        ChatDestination::GamePlayers(game_id, ..) => (chat.games_private_messages)()
            .get(&game_id)
            .cloned()
            .unwrap_or_default(),

        ChatDestination::GameSpectators(game_id, ..) => (chat.games_public_messages)()
            .get(&game_id)
            .cloned()
            .unwrap_or_default(),

        ChatDestination::User((correspondant_id, _username)) => (chat.users_messages)()
            .get(&correspondant_id)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    view! {
        <div id="ignoreChat" class="flex flex-col max-w-full h-full min-h-full">
            <div ref=div class="overflow-y-auto overflow-x-hidden w-full h-full">
                <For each=messages key=|message| message.timestamp let:message>
                    <Message message=message/>
                </For>
            </div>
            <ChatInput destination=cloned_fn()/>
        </div>
    }
}
