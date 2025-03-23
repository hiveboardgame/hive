use crate::common::HexType;
use crate::common::PieceType;
use crate::pages::play::TargetStack;
use crate::{
    common::{ActiveState, HexStack},
    components::atoms::hex::Hex,
};
use leptos::either::Either;
use leptos::prelude::*;
use leptos::{
    ev::{pointerup, touchend, touchstart},
    svg,
};
use leptos_use::{
    use_event_listener, use_event_listener_with_options, use_interval_with_options, use_window,
    UseEventListenerOptions, UseIntervalOptions,
};
use std::sync::Arc;
use web_sys::PointerEvent;

#[component]
pub fn HexStack(hex_stack: HexStack) -> impl IntoView {
    let target_stack = expect_context::<TargetStack>().0;
    let interval = StoredValue::new(Arc::new(use_interval_with_options(
        500,
        UseIntervalOptions::default().immediate(false),
    )));
    Effect::new_isomorphic(move |_| {
        if (interval.get_value().counter)() >= 1 {
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
            (interval.get_value().reset)();
            (interval.get_value().resume)();
        },
        UseEventListenerOptions::default().passive(true),
    );

    let _collapse_expand = use_event_listener_with_options(
        g_ref,
        touchend,
        move |_| {
            (interval.get_value().reset)();
            (interval.get_value().pause)();
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
                Either::Left(view! {
                    <g node_ref=g_ref>
                        <Hex hex=hex on:pointerdown=rightclick_expand />
                    </g>
                })
            } else {
                Either::Right(view! { <Hex hex=hex /> })
            }
        })
        .collect_view()
}
