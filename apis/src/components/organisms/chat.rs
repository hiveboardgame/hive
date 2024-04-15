use crate::providers::chat::Chat;
use leptos::logging::log;
use leptos::*;
use shared_types::chat_message::ChatMessage;

#[component]
pub fn Message(message: ChatMessage) -> impl IntoView {
    log!("Message is: {}", message.message);
    view! {
        <div class="flex items-center mb-4">
            <div class="flex-shrink-0 w-10 h-10 rounded-full bg-blue-500"></div>
            <div class="ml-4">
                <div class="text-sm text-gray-900">
                    {message.username} at {message.timestamp.unwrap().to_string()}
                </div>
                <div class="text-sm text-gray-900">{message.message}</div>
            </div>
        </div>
    }
}

#[component]
pub fn ChatInput() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let message = RwSignal::new(String::new());
    let input = move |evt| message.update(|v| *v = event_target_value(&evt));
    let send = move |_| {
        chat.send(
            &message.get(),
            shared_types::chat_message::ChatDestination::Lobby,
        );
    };
    view! {
        <div class="flex items-center p-4">
            <input
                type="text"
                class="flex-grow bg-gray-200 rounded-lg px-4 py-2 focus:outline-none"
                placeholder="Type your message..."
                prop:value=message
                on:input=input
            />
            <button
                on:click=send
                class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer"
            >
                Send
            </button>
        </div>
    }
}

#[component]
pub fn ChatWindow() -> impl IntoView {
    let chat = expect_context::<Chat>();
    view! {
        <div class="pt-10">
            <div class="container mx-auto px-4 py-10">
                <div class="shadow-lg rounded-lg overflow-hidden">
                    <For each=chat.lobby key=|message| message.timestamp let:message>
                        <Message message=message/>
                    </For>
                    <ChatInput/>
                </div>
            </div>
        </div>
    }
}
