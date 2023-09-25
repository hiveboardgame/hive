use leptos::*;

use crate::{atoms::hex::Hex, common::hex_stack::HexStack};

#[component]
pub fn HexStack(hex_stack: HexStack) -> impl IntoView {
    hex_stack
        .hexes
        .into_iter()
        .map(|hex| {
            view! { <Hex hex=hex/> }
        })
        .collect_view()
}
