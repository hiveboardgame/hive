use crate::common::hex::HexType;
use crate::common::piece_type::PieceType;
use crate::pages::play::TargetStack;
use crate::{
    common::{hex::ActiveState, hex_stack::HexStack},
    components::atoms::hex::Hex,
};
use leptos::ev::pointerup;
use leptos::*;
use leptos_use::{use_event_listener, use_interval_with_options, use_window, UseIntervalOptions};
use std::rc::Rc;
use web_sys::{MouseEvent, PointerEvent};

#[component]
pub fn HexStack(hex_stack: HexStack) -> impl IntoView {
    let target_stack = expect_context::<TargetStack>().0;
    let interval = store_value(Rc::new(use_interval_with_options(
        500,
        UseIntervalOptions::default().immediate(false),
    )));
    create_isomorphic_effect(move |_| {
        if (interval().counter)() >= 1 {
            target_stack.set(Some(hex_stack.position));
        }
    });
    let rightclick_expand = move |evt: PointerEvent| {
        evt.prevent_default();
        if evt.button() == 2 {
            target_stack.set(Some(hex_stack.position));
        }
    };

    let window = use_window();
    //Stop panning when user releases touch/click AND reset height adjustment on right click release
    _ = use_event_listener(window, pointerup, move |evt| {
        //is_panning.update_untracked(|b| *b = false);
        if evt.button() == 2 {
            target_stack.set(None);
        }
    });
    let longpress_expand = move |_| {
        (interval().reset)();
        (interval().resume)();
    };
    let collapse_stack = move |_| {
        (interval().reset)();
        (interval().pause)();
        target_stack.set(None);
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
                view! {
                    <Hex
                        hex=hex
                        on:pointerdown=rightclick_expand
                        on:touchstart=longpress_expand
                        on:touchend=collapse_stack
                    />
                }
            } else {
                view! { <Hex hex=hex/> }
            }
        })
        .collect_view()
}
