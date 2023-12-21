use crate::common::hex::HexType;
use crate::common::piece_type::PieceType;
use crate::pages::play::TargetStack;
use crate::{
    common::{hex::ActiveState, hex_stack::HexStack},
    components::atoms::hex::Hex,
};
use leptos::*;
use web_sys::MouseEvent;

#[component]
pub fn HexStack(hex_stack: HexStack) -> impl IntoView {
    let target_stack = expect_context::<TargetStack>().0;
    let expand_stack = move |evt: MouseEvent| {
        evt.prevent_default();
        target_stack.set(Some(hex_stack.position));
    };
    hex_stack
        .hexes
        .into_iter()
        .map(|hex| {
            let is_expandable = match hex.kind {
                HexType::Tile(_, ref piece_type) => {
                    *piece_type != PieceType::Reserve && hex.level != 0
                }
                HexType::Active(ActiveState::Board) => true,
                HexType::Target => hex.level != 0,
                _ => false,
            };

            if is_expandable {
                view! { <Hex hex=hex on:contextmenu=expand_stack/> }
            } else {
                view! { <Hex hex=hex/> }
            }
        })
        .collect_view()
}
