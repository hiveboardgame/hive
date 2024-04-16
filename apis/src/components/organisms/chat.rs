use crate::providers::chat::Chat;
use leptos::*;
use leptos_use::{use_mutation_observer_with_options, UseMutationObserverOptions};
use shared_types::chat_message::ChatMessage;

#[component]
pub fn Message(message: ChatMessage) -> impl IntoView {
    let formatted_timestamp = message
        .timestamp
        .unwrap()
        .format("%Y-%m-%d %H:%M")
        .to_string();
    view! {
        <div class="flex items-center mb-1 w-full">
            <div class="w-full px-2">
                <div class="text-sm select-text">{message.username} at {formatted_timestamp}</div>
                <div class="text-sm select-text max-w-fit break-words">{message.message}</div>
            </div>
        </div>
    }
}

#[component]
pub fn ChatInput() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let message = RwSignal::new(String::new());
    let input = move |evt| message.update(|v| *v = event_target_value(&evt));
    let send = move || {
        let the_message = message();
        if !the_message.is_empty() {
            chat.send(
                &the_message,
                shared_types::chat_message::ChatDestination::Lobby,
            );
            message.set(String::new());
        };
    };
    view! {
        <input
            type="text"
            class="bg-odd-light dark:bg-odd-dark rounded-lg px-4 py-2 focus:outline-none w-full resize-none h-auto box-border shrink-0"
            prop:value=message
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
pub fn ChatWindow() -> impl IntoView {
    let chat = expect_context::<Chat>();
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
    view! {
        <div class="h-full flex flex-col">
            <div ref=div class="overflow-y-auto h-full">
                <For each=chat.lobby key=|message| message.timestamp let:message>
                    <Message message=message/>
                </For>
            </div>
            <ChatInput/>
        </div>
    }
}
