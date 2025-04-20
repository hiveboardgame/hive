use crate::common::SvgPos;
use crate::common::TileDesign;
use crate::components::layouts::base_layout::OrientationSignal;
use crate::components::molecules::{board_pieces::BoardPieces, history_pieces::HistoryPieces};
use crate::providers::analysis::AnalysisSignal;
use crate::providers::game_state::{GameStateSignal, View};
use crate::providers::Config;
use hive_lib::GameStatus;
use leptos::either::Either;
use leptos::ev::{
    contextmenu, pointerdown, pointerleave, pointermove, pointerup, touchmove, touchstart, wheel,
};
use leptos::leptos_dom::helpers::debounce;
use leptos::{
    html,
    prelude::*,
    svg::{self, Svg},
};
use leptos_use::{
    on_click_outside, use_event_listener, use_event_listener_with_options,
    use_intersection_observer_with_options, use_resize_observer, use_throttle_fn_with_arg,
    UseEventListenerOptions, UseIntersectionObserverOptions,
};
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::{DomRectReadOnly, PointerEvent, SvgPoint, SvgRect, TouchEvent, WheelEvent};

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
pub fn Board() -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();
    let analysis = use_context::<AnalysisSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let target_stack = RwSignal::new(None);
    let config = expect_context::<Config>().0;
    let is_panning = RwSignal::new(false);
    let has_zoomed = RwSignal::new(false);
    let viewbox_signal = RwSignal::new(ViewBoxControls::new());
    let initial_touch_distance = RwSignal::<f32>::new(0.0);
    let viewbox_ref = NodeRef::<svg::Svg>::new();
    let g_ref = NodeRef::<svg::G>::new();
    let div_ref = NodeRef::<html::Div>::new();
    let zoom_in_limit = 150.0;
    let zoom_out_limit = 2500.0;
    let last_turn = game_state.is_last_turn_as_signal();
    let board_view = create_read_slice(game_state.signal, |gs| gs.view.clone());
    let game_status = create_read_slice(game_state.signal, |gs| gs.state.game_status.clone());
    let board_style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex grow min-h-0"
        } else {
            "col-span-8 row-span-6"
        }
    };
    let history_style = move || match board_view() {
        View::Game => "",
        View::History => match game_status() {
            GameStatus::Finished(_) => "",
            _ => {
                if last_turn() {
                    ""
                } else {
                    "sepia-[.75]"
                }
            }
        },
    };

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

    let current_center = move || {
        game_state
            .signal
            .get_untracked()
            .state
            .board
            .center_coordinates()
    };

    let straight = Signal::derive(move || config().tile.design == TileDesign::ThreeD);
    let tile_opts = Signal::derive(move || config().tile);
    Effect::watch(
        move || (),
        move |_, _, _| {
            let div = div_ref.get_untracked().expect("it exists");
            let rect = div.get_bounding_client_rect();
            let svg_pos = SvgPos::center_for_level(current_center(), 0, straight.get_untracked());
            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.x = 0.0;
                viewbox_controls.y = 0.0;
                viewbox_controls.width = rect.width() as f32;
                viewbox_controls.height = rect.height() as f32;
                viewbox_controls.x_transform = -(svg_pos.0 - (viewbox_controls.width / 2.0));
                viewbox_controls.y_transform = -(svg_pos.1 - (viewbox_controls.height / 2.0));
            });
        },
        true,
    );

    //This handles board resizes
    let throttled_resize = use_throttle_fn_with_arg(
        move |rect: DomRectReadOnly| {
            let svg_pos = SvgPos::center_for_level(current_center(), 0, straight.get_untracked());
            let svg = viewbox_ref.get_untracked().expect("It exists");
            // if user has zoomed, keep their scale when resizing board
            let (current_x_scale, current_y_scale) = if has_zoomed.get_untracked() {
                (
                    svg.client_width() as f32 / viewbox_signal.get_untracked().width,
                    svg.client_height() as f32 / viewbox_signal.get_untracked().height,
                )
            } else {
                (1.0, 1.0)
            };

            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.x = 0.0;
                viewbox_controls.y = 0.0;
                viewbox_controls.width = rect.width() as f32 / current_x_scale;
                viewbox_controls.height = rect.height() as f32 / current_y_scale;
                viewbox_controls.x_transform = -(svg_pos.0 - (viewbox_controls.width / 2.0));
                viewbox_controls.y_transform = -(svg_pos.1 - (viewbox_controls.height / 2.0));
            });
        },
        10.0,
    );
    use_resize_observer(div_ref, move |entries, _observer| {
        let rect = entries[0].content_rect();
        throttled_resize(rect);
    });

    let is_visible = RwSignal::new(true);
    _ = use_intersection_observer_with_options(
        g_ref,
        move |entries, _| {
            is_visible.set(entries[0].is_intersecting());
        },
        UseIntersectionObserverOptions::default()
            .root(Some(viewbox_ref))
            .thresholds(vec![0.5]),
    );

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
        if is_panning.get_untracked() && target_stack.with_untracked(|v| v.is_none()) {
            let moved_point = svg_point_from_pointer(viewbox_ref, &evt);
            let g_bbox = get_bbox(g_ref);
            let mut future_viewbox = viewbox_signal.get_untracked();
            future_viewbox.x -= moved_point.x() - future_viewbox.drag_start_x;
            future_viewbox.y -= moved_point.y() - future_viewbox.drag_start_y;
            if is_visible() {
                viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                    viewbox_controls.x = future_viewbox.x;
                    viewbox_controls.y = future_viewbox.y
                });
            } else if will_svg_be_visible(g_bbox, &future_viewbox) {
                viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                    viewbox_controls.x = future_viewbox.x;
                    viewbox_controls.y = future_viewbox.y
                });
            };
        }
    });

    _ = use_event_listener_with_options(
        viewbox_ref,
        wheel,
        debounce(Duration::from_millis(7), move |evt: WheelEvent| {
            if !is_panning.get_untracked() {
                let initial_point = svg_point_from_wheel(viewbox_ref, &evt);
                let scale: f32 = if evt.delta_y() > 0.0 { 0.09 } else { -0.09 };
                let g_bbox = get_bbox(g_ref);
                let mut future_viewbox = viewbox_signal.get_untracked();
                let initial_height = future_viewbox.height;
                let initial_width = future_viewbox.width;
                future_viewbox.width += initial_width * scale;
                future_viewbox.height += initial_height * scale;
                future_viewbox.x = initial_point.x()
                    - (initial_point.x() - future_viewbox.x) / initial_width * future_viewbox.width;
                future_viewbox.y = initial_point.y()
                    - (initial_point.y() - future_viewbox.y) / initial_height
                        * future_viewbox.height;
                if (scale < 0.0 && initial_height >= zoom_in_limit)
                    || (scale > 0.0 && initial_height <= zoom_out_limit)
                {
                    if is_visible() {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox;
                        });
                        has_zoomed.set(true);
                    } else if will_svg_be_visible(g_bbox, &future_viewbox) {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox;
                        });
                        has_zoomed.set(true);
                    }
                }
            }
        }),
        UseEventListenerOptions::default().passive(true),
    );

    //Zoom on pinch
    _ = use_event_listener_with_options(
        viewbox_ref,
        touchstart,
        debounce(Duration::from_millis(1), move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                is_panning.update_untracked(|b| *b = false);
                let initial_point_0 = svg_point_from_touch(viewbox_ref, &evt, 0);
                let initial_point_1 = svg_point_from_touch(viewbox_ref, &evt, 1);
                initial_touch_distance
                    .update(move |v| *v = get_touch_distance(initial_point_0, initial_point_1));
            }
        }),
        UseEventListenerOptions::default().passive(true),
    );

    _ = use_event_listener_with_options(
        viewbox_ref,
        touchmove,
        debounce(Duration::from_millis(1), move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                let current_point_0 = svg_point_from_touch(viewbox_ref, &evt, 0);
                let current_point_1 = svg_point_from_touch(viewbox_ref, &evt, 1);
                let current_distance = get_touch_distance(current_point_0.clone(), current_point_1);
                let scale = current_distance / initial_touch_distance();
                let g_bbox = get_bbox(g_ref);
                let mut future_viewbox = viewbox_signal.get_untracked();
                let intermediate_height = future_viewbox.height / scale;
                future_viewbox.width /= scale;
                future_viewbox.height /= scale;
                future_viewbox.x =
                    current_point_0.x() - (current_point_0.x() - future_viewbox.x) / scale;
                future_viewbox.y =
                    current_point_0.y() - (current_point_0.y() - future_viewbox.y) / scale;
                if intermediate_height >= zoom_in_limit && intermediate_height <= zoom_out_limit {
                    if is_visible() {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox
                        });
                        has_zoomed.set(true);
                    } else if will_svg_be_visible(g_bbox, &future_viewbox) {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox
                        });
                        has_zoomed.set(true);
                    }
                };
            }
        }),
        UseEventListenerOptions::default().passive(true),
    );

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

    _ = on_click_outside(g_ref, move |event| {
        let clicked_timer = event
            .target()
            .map(|t| t.dyn_ref::<web_sys::HtmlElement>().map(|t| t.id()))
            .map(|id| id.is_some_and(|id| id == "timer"));
        if !clicked_timer.unwrap_or_default() {
            game_state.reset();
        }
    });
    view! {
        <div node_ref=div_ref class=board_style>

            <svg
                width="100%"
                height="100%"
                viewBox=viewbox_string
                class=move || format!("touch-none duration-300 {}", history_style())
                node_ref=viewbox_ref
                xmlns="http://www.w3.org/2000/svg"
            >
                <g transform=transform node_ref=g_ref>
                    {move || {
                        if board_view() == View::History && !last_turn() && analysis.is_none() {
                            Either::Left(
                                view! { <HistoryPieces tile_opts=tile_opts() target_stack /> },
                            )
                        } else {
                            Either::Right(
                                view! { <BoardPieces tile_opts=tile_opts() target_stack /> },
                            )
                        }
                    }}
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
    (distance_x * distance_x + distance_y * distance_y).sqrt()
}

fn will_svg_be_visible(bbox: SvgRect, viewbox: &ViewBoxControls) -> bool {
    let bbox_mid_x = bbox.x() + viewbox.x_transform + bbox.width() / 2.0;
    let bbox_mid_y = bbox.y() + viewbox.y_transform + bbox.height() / 2.0;
    let viewbox_right = viewbox.x + viewbox.width;
    let viewbox_bottom = viewbox.y + viewbox.height;

    (bbox_mid_x > viewbox.x)
        && (bbox_mid_x < viewbox_right)
        && (bbox_mid_y > viewbox.y)
        && (bbox_mid_y < viewbox_bottom)
}

fn get_bbox(g_ref: NodeRef<svg::G>) -> SvgRect {
    g_ref
        .get_untracked()
        .expect("G exists")
        .unchecked_ref::<web_sys::SvgGraphicsElement>()
        .get_b_box()
        .expect("Rect")
}
