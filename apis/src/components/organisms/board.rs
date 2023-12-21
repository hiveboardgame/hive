use crate::common::svg_pos::SvgPos;
use crate::providers::game_state::{GameStateSignal, View};
use crate::{
    components::{
        atoms::svgs::Svgs,
        molecules::{board_pieces::BoardPieces, history_pieces::HistoryPieces},
    },
    pages::play::TargetStack,
};
use hive_lib::position::Position;
use leptos::ev::{
    contextmenu, pointerdown, pointerleave, pointermove, pointerup, touchmove, touchstart, wheel,
};
use leptos::svg::Svg;
use leptos::*;
use leptos_use::{use_event_listener, use_resize_observer};
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, SvgPoint, TouchEvent, WheelEvent};

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
    let game_state_signal = expect_context::<GameStateSignal>();
    let target_stack = expect_context::<TargetStack>().0;
    let is_panning = create_rw_signal(false);
    let viewbox_signal = create_rw_signal(ViewBoxControls::new());
    let initial_touch_distance = create_rw_signal::<f32>(0.0);
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

    //on load and resize make sure to resize the viewbox
    use_resize_observer(div_ref, move |entries, _observer| {
        let rect = entries[0].content_rect();
        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
            viewbox_controls.width = rect.width() as f32;
            viewbox_controls.height = rect.height() as f32;
            viewbox_controls.x_transform = -(svg_pos.0 - (rect.width() as f32 / 2.0));
            viewbox_controls.y_transform = -(svg_pos.1 - (rect.height() as f32 / 2.0));
        });
    });

    //Start panning and record point where it starts for mouse on left mouse button hold and touch
    _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
        if evt.button() == 0 {
            is_panning.update_untracked(|b| *b = true);
            let ref_point = svg_point_from_pointer(viewbox_ref, &evt);
            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.drag_start_x = ref_point.x();
                viewbox_controls.drag_start_y = ref_point.y();
            });
        }
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

    //Zoom on pinch
    _ = use_event_listener(viewbox_ref, touchstart, move |evt| {
        if evt.touches().length() == 2 {
            is_panning.update_untracked(|b| *b = false);
            let initial_point_0 = svg_point_from_touch(viewbox_ref, &evt, 0);
            let initial_point_1 = svg_point_from_touch(viewbox_ref, &evt, 1);
            initial_touch_distance
                .update(move |v| *v = get_touch_distance(initial_point_0, initial_point_1));
        }
    });

    _ = use_event_listener(viewbox_ref, touchmove, move |evt| {
        if evt.touches().length() == 2 {
            let current_point_0 = svg_point_from_touch(viewbox_ref, &evt, 0);
            let current_point_1 = svg_point_from_touch(viewbox_ref, &evt, 1);
            let current_distance = get_touch_distance(current_point_0.clone(), current_point_1);
            let scale = current_distance / initial_touch_distance();
            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.width /= scale;
                viewbox_controls.height /= scale;
                viewbox_controls.x =
                    current_point_0.x() - (current_point_0.x() - viewbox_controls.x) / scale;
                viewbox_controls.y =
                    current_point_0.y() - (current_point_0.y() - viewbox_controls.y) / scale;
            });
        }
    });

    //Stop panning when user releases touch/click AND reset height adjustment on right click release
    _ = use_event_listener(viewbox_ref, pointerup, move |_| {
        is_panning.update_untracked(|b| *b = false);
        target_stack.set(None);
    });

    //Stop panning when pointer leaves board area
    _ = use_event_listener(viewbox_ref, pointerleave, move |_| {
        is_panning.update_untracked(|b| *b = false);
    });

    //Prevent right click/context menu on board
    _ = use_event_listener(viewbox_ref, contextmenu, move |evt| {
        evt.prevent_default();
    });

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
                            View::History == (game_state_signal.signal)().view
                                && !(game_state_signal.signal)().is_last_turn()
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

fn svg_point_from_touch(svg: NodeRef<Svg>, evt: &TouchEvent, ind: u32) -> web_sys::SvgPoint {
    svg_point_from_coordinates(
        svg,
        evt.touches()
            .get(ind)
            .expect("It was called by a valid touch event")
            .client_x() as f32,
        evt.touches()
            .get(ind)
            .expect("It was called by a valid touch event")
            .client_y() as f32,
    )
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

fn get_touch_distance(point_0: SvgPoint, point_1: SvgPoint) -> f32 {
    let distance_x = point_0.x() - point_1.x();
    let distance_y = point_0.y() - point_1.y();
    (distance_x * distance_x + distance_y + distance_y).sqrt()
}
