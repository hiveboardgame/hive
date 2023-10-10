use crate::{common::hex_stack::HexStack, components::atoms::hex::Hex};
use leptos::*;

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
