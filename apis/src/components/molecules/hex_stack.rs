use crate::common::HexType;
use crate::common::PieceType;
use crate::pages::play::TargetStack;
use crate::{
    common::{ActiveState, HexStack},
    components::atoms::hex::Hex,
};
use leptos::ev::{pointerup, touchend, touchstart};
use leptos::*;
use leptos_use::{
    use_event_listener, use_event_listener_with_options, use_interval_with_options, use_window,
    UseEventListenerOptions, UseIntervalOptions,
};
use std::rc::Rc;
use web_sys::PointerEvent;

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
    _ = use_event_listener(window, pointerup, move |evt| {
        if evt.button() == 2 {
            target_stack.set(None);
        }
    });
    let g_ref = NodeRef::<svg::G>::new();
    let _longpress_expand = use_event_listener_with_options(
        g_ref,
        touchstart,
        move |_| {
            (interval().reset)();
            (interval().resume)();
        },
        UseEventListenerOptions::default().passive(true),
    );

    let _collapse_expand = use_event_listener_with_options(
        g_ref,
        touchend,
        move |_| {
            (interval().reset)();
            (interval().pause)();
            target_stack.set(None);
        },
        UseEventListenerOptions::default().passive(true),
    );

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
                    <g ref=g_ref>
                        <Hex hex=hex on:pointerdown=rightclick_expand />
                    </g>
                }
                .into_view()
            } else {
                view! { <Hex hex=hex /> }
            }
        })
        .collect_view()
}
