use crate::{
    components::update_from_event::update_from_input,
    providers::{AuthContext, ClientApi, chat::Chat, game_state::GameStateSignal},
};
use chrono::Local;
use leptos::{html, prelude::*};
use leptos_router::hooks::use_params_map;
use leptos_use::{use_mutation_observer_with_options, UseMutationObserverOptions};
use shared_types::{ChatDestination, ChatMessage, GameId, SimpleDestination};
use uuid::Uuid;

#[component]
pub fn Message(message: ChatMessage) -> impl IntoView {
    let user_local_time = message
        .timestamp
        .unwrap()
        .with_timezone(&Local)
        .format(" %d/%m/%Y %H:%M %Z")
        .to_string();
    let turn = message.turn.map(|turn| format!(" on turn {turn}:"));

    view! {
        <div class="flex flex-col items-start mb-1 w-full">
            <div class="flex gap-1 px-2">
                <div class="font-bold">{message.username}</div>
                {user_local_time}
                {turn}
            </div>
            <div class="px-2 w-full break-words max-w-fit">{message.message}</div>
        </div>
    }
}

#[component]
pub fn ChatInput(destination: Signal<ChatDestination>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let game_state = use_context::<GameStateSignal>();
    let api = expect_context::<ClientApi>();
    let turn = move || game_state.map(|gs| gs.signal.with(|state| state.state.turn));
    let send = move || {
        let message = chat.typed_message.get();
        if !message.is_empty() {
            chat.send(&message, destination(), turn(), api);
            chat.typed_message.set(String::new());
        };
    };
    let placeholder = move || match destination() {
        ChatDestination::GamePlayers(_, _, _) => "Chat with opponent",
        ChatDestination::GameSpectators(_, _, _) => "Chat with spectators",
        _ => "Chat",
    };
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    view! {
        <input
            node_ref=my_input
            type="text"
            class="box-border px-2 py-4 w-full rounded-lg resize-none bg-odd-light dark:bg-odd-dark focus:outline-none shrink-0"
            prop:value=chat.typed_message
            prop:placeholder=placeholder
            on:input=update_from_input(chat.typed_message)
            on:keydown=move |evt| {
                if evt.key() == "Enter" {
                    evt.prevent_default();
                    send();
                }
            }

            maxlength="1000"
        />
    }
}

#[component]
pub fn ChatWindow(
    destination: SimpleDestination,
    #[prop(optional)] correspondant_id: Option<Uuid>,
    #[prop(optional)] correspondant_username: String,
) -> impl IntoView {
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let game_state = expect_context::<GameStateSignal>();
    let uid = auth_context
        .user
        .with_untracked(|a| a.as_ref().map(|user| user.user.uid));
    let white_id = create_read_slice(game_state.signal, |gs| gs.white_id);
    let black_id = create_read_slice(game_state.signal, |gs| gs.black_id);
    let white_id = move || white_id().expect("Game has white player");
    let black_id = move || black_id().expect("Game has black player");

    let correspondant_id = Signal::derive(move || correspondant_id.map_or(Uuid::new_v4(), |id| id));
    let correspondant_username = Signal::derive(move || correspondant_username.clone());
    let div = NodeRef::<html::Div>::new();
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

    let actual_destination = Signal::derive(move || match destination.clone() {
        SimpleDestination::Game => {
            if game_state.signal.with(|gs| gs.uid_is_player(uid)) {
                ChatDestination::GamePlayers(game_id(), white_id(), black_id())
            } else {
                ChatDestination::GameSpectators(game_id(), white_id(), black_id())
            }
        }
        SimpleDestination::User => {
            ChatDestination::User((correspondant_id(), correspondant_username()))
        }
        SimpleDestination::Global => ChatDestination::Global,
        SimpleDestination::Tournament(tournament_id) => {
            ChatDestination::TournamentLobby(tournament_id)
        }
    });
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
        <div
            id="ignoreChat"
            class="flex flex-col flex-grow justify-between w-full min-w-full max-w-full h-full"
        >
            <div node_ref=div class="overflow-y-auto flex-grow w-full min-w-full h-0">
                <For each=messages key=|message| message.timestamp let:message>
                    <Message message=message />
                </For>
            </div>
            <ChatInput destination=actual_destination />
        </div>
    }
}
