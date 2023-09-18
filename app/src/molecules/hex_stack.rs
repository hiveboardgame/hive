use leptos::*;

use crate::{atoms::hex::Hex, common::hex_stack::HexStack};

#[component]
pub fn HexStack(cx: Scope, hex_stack: HexStack) -> impl IntoView {
    hex_stack
        .hexes
        .into_iter()
        .map(|hex| {
            view! { cx, <Hex hex=hex/> }
        })
        .collect_view(cx)
}
