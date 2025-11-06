use crate::common::ClientRequest;
use leptos::prelude::*;
use shared_types::ChatMessageContainer;

#[derive(Clone)]
pub struct ApiRequests {
}

#[derive(Clone)]
pub struct ApiRequestsProvider(pub Signal<ApiRequests>);

impl ApiRequests {
    pub fn chat(&self, message: &ChatMessageContainer) {
        let msg = ClientRequest::Chat(message.to_owned());
        //self.websocket.send(&msg);
    }
}

pub fn provide_api_requests() {
    let api_requests = ApiRequests{};
    provide_context(ApiRequestsProvider(Signal::derive(move || {
        api_requests.clone()
    })));
}
