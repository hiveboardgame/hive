use crate::common::HexType;
use crate::common::PieceType;
use crate::providers::config::TileOptions;
use crate::{
    common::{ActiveState, HexStack},
    components::atoms::hex::Hex,
};
use hive_lib::Position;
use leptos::either::Either;
use leptos::ev::touchstart;
use leptos::prelude::*;
use leptos::{
    ev::{pointerup, touchend},
    svg,
};
use leptos_use::{
    use_event_listener, use_event_listener_with_options, use_timeout_fn, use_window,
    UseEventListenerOptions, UseTimeoutFnReturn,
};

#[component]
pub fn HexStack(
    hex_stack: HexStack,
    tile_opts: TileOptions,
    target_stack: RwSignal<Option<Position>>,
) -> impl IntoView {
    let tile_opts = StoredValue::new(tile_opts);
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
                // Mouse right click to expand
                let window = use_window();
                _ = use_event_listener(window.clone(), pointerup, move |evt| {
                    if evt.button() == 2 {
                        target_stack.set(None);
                    }
                });
                // Touch longpress to expand
                let UseTimeoutFnReturn { start, stop, .. } = use_timeout_fn(
                    move |pos| {
                        target_stack.set(pos);
                    },
                    500.0,
                );
                let g_ref = NodeRef::<svg::G>::new();
                let _ = use_event_listener_with_options(
                    g_ref,
                    touchstart,
                    move |_| start(Some(hex_stack.position)),
                    UseEventListenerOptions::default().passive(true),
                );
                let _ = use_event_listener_with_options(
                    window,
                    touchend,
                    move |_| {
                        stop();
                        target_stack.set(None);
                    },
                    UseEventListenerOptions::default().passive(true),
                );
                Either::Left(view! {
                    <g node_ref=g_ref>
                        <Hex
                            hex=hex
                            on:pointerdown=move |evt| {
                                evt.prevent_default();
                                if evt.button() == 2 {
                                    target_stack.set(Some(hex_stack.position));
                                }
                            }
                            tile_opts=tile_opts.get_value()
                            target_stack
                        />
                    </g>
                })
            } else {
                Either::Right(
                    view! { <Hex hex=hex tile_opts=tile_opts.get_value() target_stack /> },
                )
            }
        })
        .collect_view()
}
