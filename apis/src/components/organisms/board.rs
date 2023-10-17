use crate::common::svg_pos::SvgPos;
use crate::components::{
    atoms::svgs::Svgs,
    molecules::{board_pieces::BoardPieces, history_pieces::HistoryPieces},
};
use crate::providers::game_state::{GameStateSignal, View};
use hive_lib::position::Position;
use leptos::ev::{contextmenu, pointerdown, pointerleave, pointermove, pointerup, wheel};
use leptos::logging::*;
use leptos::svg::Svg;
use leptos::*;
use leptos_use::use_event_listener;
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, WheelEvent};

#[derive(Debug, Clone)]
struct ViewBoxControls {
    // The coordinates svg_pos gives to start spawn position at 16 16 that will scale based on initial size of the viewbox
    x_transform: f32,
    y_transform: f32,
    // Min_x, min_y, width and height to be used for the viewbox
    x: f32,
    y: f32,
    height: f32,
    width: f32,
    // Panning numbers
    drag_start_x: f32,
    drag_start_y: f32,
}

impl ViewBoxControls {
    pub fn new() -> Self {
        ViewBoxControls {
            x_transform: 0.0,
            y_transform: 0.0,
            x: 0.0,
            y: 0.0,
            height: 550.0,
            width: 550.0,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
        }
    }
}

#[component]
pub fn Board(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] overwrite_tw_classes: &'static str,
) -> impl IntoView {
    let is_panning = create_rw_signal(false);
    let viewbox_signal = create_rw_signal(ViewBoxControls::new());
    let viewbox_ref = create_node_ref::<svg::Svg>();
    let div_ref = create_node_ref::<html::Div>();

    let viewbox_string = move || {
        format!(
            "{} {} {} {}",
            viewbox_signal().x,
            viewbox_signal().y,
            viewbox_signal().width,
            viewbox_signal().height
        )
    };

    let transform = move || {
        format!(
            "translate({},{})",
            viewbox_signal().x_transform,
            viewbox_signal().y_transform
        )
    };

    let initial_position = Position::initial_spawn_position();
    let svg_pos = SvgPos::center_for_level(initial_position, 0);

    div_ref.on_load(move |_| {
        create_effect(move |_| {
            //Make initial piece spawn in center of board
            let div = div_ref.get_untracked().expect("Div should already exist");
            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.width = div.offset_width() as f32;
                viewbox_controls.height = div.offset_height() as f32;
                viewbox_controls.x_transform = -(svg_pos.0 - (div.offset_width() as f32 / 2.0));
                viewbox_controls.y_transform = -(svg_pos.1 - (div.offset_height() as f32 / 2.0));
            });

            //Start panning and record point where it starts for mouse and touch
            _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
                is_panning.update_untracked(|b| *b = true);
                let ref_point = svg_point_from_pointer(viewbox_ref, &evt);
                viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                    viewbox_controls.drag_start_x = ref_point.x();
                    viewbox_controls.drag_start_y = ref_point.y();
                });
            });

            //Keep panning while user drags around
            _ = use_event_listener(viewbox_ref, pointermove, move |evt| {
                if is_panning.get_untracked() {
                    let moved_point = svg_point_from_pointer(viewbox_ref, &evt);
                    viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                        viewbox_controls.x -= moved_point.x() - viewbox_controls.drag_start_x;
                        viewbox_controls.y -= moved_point.y() - viewbox_controls.drag_start_y;
                    })
                }
            });

            //Zoom on point with mousewheel/touchpad
            _ = use_event_listener(viewbox_ref, wheel, move |evt| {
                evt.prevent_default();
                if !is_panning.get_untracked() {
                    let initial_point = svg_point_from_wheel(viewbox_ref, &evt);
                    let scale: f32 = if evt.delta_y() > 0.0 { 0.09 } else { -0.09 };
                    viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                        let initial_width = viewbox_controls.width;
                        let initial_height = viewbox_controls.height;
                        viewbox_controls.width += initial_width * scale;
                        viewbox_controls.height += initial_height * scale;
                        viewbox_controls.x = initial_point.x()
                            - (initial_point.x() - viewbox_controls.x) / initial_width
                                * viewbox_controls.width;
                        viewbox_controls.y = initial_point.y()
                            - (initial_point.y() - viewbox_controls.y) / initial_height
                                * viewbox_controls.height;
                    });
                }
            });

            //Stop panning when user releases touch/click
            _ = use_event_listener(viewbox_ref, pointerup, move |_| {
                is_panning.update_untracked(|b| *b = false);
            });

            //Stop panning when pointer leaves board area
            _ = use_event_listener(viewbox_ref, pointerleave, move |_| {
                is_panning.update_untracked(|b| *b = false);
            });

            //Prevent right click/context menu on board
            _ = use_event_listener(viewbox_ref, contextmenu, move |evt| {
                evt.prevent_default();
            });
        });
    });

    let game_state_signal = expect_context::<GameStateSignal>();

    view! {
        <div
            ref=div_ref
            class=if !overwrite_tw_classes.is_empty() {
                overwrite_tw_classes.to_string()
            } else {
                format!("col-span-8 row-span-6 h-full w-full {extend_tw_classes}")
            }
        >

            <svg
                viewBox=viewbox_string
                class="touch-none"
                ref=viewbox_ref
                xmlns="http://www.w3.org/2000/svg"
            >
                <Svgs/>
                <g transform=transform>
                    <Show
                        when=move || {
                            View::History == game_state_signal.signal.get().view
                                && !game_state_signal.signal.get().is_last_turn()
                        }

                        fallback=|| {
                            view! { <BoardPieces/> }
                        }
                    >

                        <HistoryPieces/>
                    </Show>
                </g>
            </svg>
        </div>
    }
}

fn svg_point_from_pointer(svg: NodeRef<Svg>, evt: &PointerEvent) -> web_sys::SvgPoint {
    svg_point_from_coordinates(svg, evt.x() as f32, evt.y() as f32)
}

fn svg_point_from_wheel(svg: NodeRef<Svg>, evt: &WheelEvent) -> web_sys::SvgPoint {
    svg_point_from_coordinates(svg, evt.x() as f32, evt.y() as f32)
}

fn svg_point_from_coordinates(svg: NodeRef<Svg>, x: f32, y: f32) -> web_sys::SvgPoint {
    let svg = svg.get_untracked().expect("svg should exist already");
    let svg_graphics_element = svg.unchecked_ref::<web_sys::SvgGraphicsElement>();
    let svg_svg_element = svg.unchecked_ref::<web_sys::SvgsvgElement>();
    let point: web_sys::SvgPoint = svg_svg_element.create_svg_point();
    point.set_x(x);
    point.set_y(y);

    point.matrix_transform(
        &svg_graphics_element
            .get_screen_ctm()
            .expect("screen ctm missing")
            .inverse()
            .expect("matrix not inversed"),
    )
}
