use crate::common::hex_stack::HexStack;
use crate::components::atoms::simple_hex::SimpleHex;
use leptos::*;

#[component]
pub fn SimpleHexStack(hex_stack: HexStack) -> impl IntoView {
    hex_stack
        .hexes
        .into_iter()
        .map(|hex| {
            view! { <SimpleHex hex=hex/> }
        })
        .collect_view()
}
