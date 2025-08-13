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
use leptos::{
    html,
    prelude::*,
    svg,
};
use leptos_use::{
    on_click_outside, use_event_listener, use_event_listener_with_options,
    use_intersection_observer_with_options, use_raf_fn, use_resize_observer,
    UseEventListenerOptions, UseIntersectionObserverOptions,
};
use wasm_bindgen::JsCast;
use web_sys::{TouchEvent, WheelEvent};

#[derive(Debug, Clone)]
enum ViewBoxUpdateType {
    Resize {
        width: f32,
        height: f32,
    },
    Pan {
        delta_x: f32,
        delta_y: f32,
    },
    Zoom {
        center_x: f32,
        center_y: f32,
        scale: f32,
    },
}

#[derive(Debug, Clone)]
struct ViewBoxControls {
    // ViewBox bounds (min_x, min_y, width, height for the viewbox)
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    // Transform coordinates (svg_pos gives start spawn position at 16,16 that scales based on initial viewbox size)
    x_transform: f32,
    y_transform: f32,
    // Panning state (drag start coordinates)
    drag_start_x: f32,
    drag_start_y: f32,
}

impl ViewBoxControls {
    pub fn new() -> Self {
        ViewBoxControls {
            x: 0.0,
            y: 0.0,
            width: 550.0,
            height: 550.0,
            x_transform: 0.0,
            y_transform: 0.0,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
        }
    }
    
    fn calculate_zoom(mut self, center_x: f32, center_y: f32, scale: f32) -> Self {
        self.width /= scale;
        self.height /= scale;
        self.x = center_x - (center_x - self.x) / scale;
        self.y = center_y - (center_y - self.y) / scale;
        self
    }
    
    fn calculate_pan(mut self, delta_x: f32, delta_y: f32) -> Self {
        self.x -= delta_x;
        self.y -= delta_y;
        self
    }
}


struct ViewBoxState {
    is_panning: RwSignal<bool>,
    has_zoomed: RwSignal<bool>,
    is_visible: RwSignal<bool>,
    zoom_in_limit: f32,
    zoom_out_limit: f32,
}

impl ViewBoxState {
    fn new() -> Self {
        Self {
            is_panning: RwSignal::new(false),
            has_zoomed: RwSignal::new(false),
            is_visible: RwSignal::new(true),
            zoom_in_limit: 150.0,
            zoom_out_limit: 2500.0,
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
    let viewbox_state = ViewBoxState::new();
    let viewbox_signal = RwSignal::new(ViewBoxControls::new());
    let initial_touch_distance = RwSignal::<f32>::new(0.0);
    let pending_update = RwSignal::new(None::<ViewBoxUpdateType>);
    let viewbox_ref = NodeRef::<svg::Svg>::new();
    let g_ref = NodeRef::<svg::G>::new();
    let div_ref = NodeRef::<html::Div>::new();
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
            GameStatus::Finished(_) | GameStatus::Adjudicated => "",
            _ => {
                if last_turn() {
                    ""
                } else {
                    "sepia-[.75]"
                }
            }
        },
    };

    let viewbox_string =
        move || viewbox_signal.with(|vb| format!("{} {} {} {}", vb.x, vb.y, vb.width, vb.height));

    let transform = move || {
        viewbox_signal.with(|vb| format!("translate({},{})", vb.x_transform, vb.y_transform))
    };

    let current_center = 
        game_state
            .signal
            .with_untracked(|gs| gs.state.board.center_coordinates());

    let straight = config.with_untracked(|c| c.tile.design == TileDesign::ThreeD);
    let tile_opts = Signal::derive(move || config.with(|c| c.tile.clone()));

    let background_style = Signal::derive(move || {
        let bg = config.with(|c| c.tile.get_effective_background_color(c.prefers_dark));
        format!("background-color: {bg}")
    });
    

    // Unified RAF-based viewbox update system
    let update_viewbox_size = move |width: f32, height: f32, respect_zoom: bool| {
        let svg_pos = SvgPos::center_for_level(current_center, 0, straight);
        let (scale_x, scale_y) = if respect_zoom && viewbox_state.has_zoomed.get_untracked() {
            let svg = viewbox_ref.get_untracked().expect("It exists");
            viewbox_signal.with_untracked(|vb| {
                (
                    svg.client_width() as f32 / vb.width,
                    svg.client_height() as f32 / vb.height,
                )
            })
        } else {
            (1.0, 1.0)
        };

        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
            viewbox_controls.x = 0.0;
            viewbox_controls.y = 0.0;
            viewbox_controls.width = width / scale_x;
            viewbox_controls.height = height / scale_y;
            viewbox_controls.x_transform = -(svg_pos.0 - (viewbox_controls.width / 2.0));
            viewbox_controls.y_transform = -(svg_pos.1 - (viewbox_controls.height / 2.0));
        });
    };

    let raf_controller = use_raf_fn(move |_| {
        let update = pending_update.get_untracked();
        if let Some(update) = update {
            pending_update.set(None);
            match update {
                ViewBoxUpdateType::Resize { width, height } => {
                    update_viewbox_size(width, height, true);
                }
                ViewBoxUpdateType::Pan { delta_x, delta_y } => {
                    if viewbox_state.is_panning.get_untracked() && target_stack.with_untracked(|v| v.is_none()) {
                        let future_viewbox = viewbox_signal.get_untracked().calculate_pan(delta_x, delta_y);
                        if viewbox_state.is_visible.get() || will_svg_be_visible(g_ref, &future_viewbox) {
                            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                                viewbox_controls.x = future_viewbox.x;
                                viewbox_controls.y = future_viewbox.y;
                            });
                        }
                    }
                }
                ViewBoxUpdateType::Zoom {
                    center_x,
                    center_y,
                    scale,
                } => {
                    let future_viewbox = viewbox_signal.get_untracked().calculate_zoom(center_x, center_y, scale);
                    let intermediate_height = future_viewbox.height;

                    if (intermediate_height >= viewbox_state.zoom_in_limit && intermediate_height <= viewbox_state.zoom_out_limit)
                        && (viewbox_state.is_visible.get() || will_svg_be_visible(g_ref, &future_viewbox))
                    {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox;
                        });
                        viewbox_state.has_zoomed.set(true);
                    }
                }
            }
        }
    });

    let queue_update = StoredValue::new(move |update_type: ViewBoxUpdateType| {
        let was_empty = pending_update.get_untracked().is_none();
        pending_update.set(Some(update_type));
        if was_empty {
            (raf_controller.resume)();
        }
    });
    Effect::watch(
        move || (),
        move |_, _, _| {
            let div = div_ref.get_untracked().expect("it exists");
            let rect = div.get_bounding_client_rect();
            update_viewbox_size(rect.width() as f32, rect.height() as f32, false);
        },
        true,
    );

    //This handles board resizes
    use_resize_observer(div_ref, move |entries, _observer| {
        let rect = entries[0].content_rect();
        queue_update.with_value(|f| f(ViewBoxUpdateType::Resize {
            width: rect.width() as f32,
            height: rect.height() as f32,
        }));
    });

    _ = use_intersection_observer_with_options(
        g_ref,
        move |entries, _| {
            viewbox_state.is_visible.set(entries[0].is_intersecting());
        },
        UseIntersectionObserverOptions::default()
            .root(Some(viewbox_ref))
            .thresholds(vec![0.5]),
    );

    //Start panning and record point where it starts for mouse on left mouse button hold and touch
    _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
        if evt.button() == 0 {
            viewbox_state.is_panning.update_untracked(|b| *b = true);
            let (x, y) = screen_to_svg_coordinates(viewbox_ref, evt.x() as f32, evt.y() as f32);
            viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                viewbox_controls.drag_start_x = x;
                viewbox_controls.drag_start_y = y;
            });
        }
    });

    //Keep panning while user drags around
    _ = use_event_listener(viewbox_ref, pointermove, move |evt| {
        if viewbox_state.is_panning.get_untracked() && target_stack.with_untracked(|v| v.is_none()) {
            let (x, y) = screen_to_svg_coordinates(viewbox_ref, evt.x() as f32, evt.y() as f32);
            let current_viewbox = viewbox_signal.get_untracked();
            let delta_x = x - current_viewbox.drag_start_x;
            let delta_y = y - current_viewbox.drag_start_y;
            queue_update.with_value(|f| f(ViewBoxUpdateType::Pan { delta_x, delta_y }));
        }
    });

    _ = use_event_listener_with_options(
        viewbox_ref,
        wheel,
        move |evt: WheelEvent| {
            if !viewbox_state.is_panning.get_untracked() {
                let (x, y) = screen_to_svg_coordinates(viewbox_ref, evt.x() as f32, evt.y() as f32);
                let scale: f32 = if evt.delta_y() > 0.0 { 1.09 } else { 0.91 };
                queue_update.with_value(|f| f(ViewBoxUpdateType::Zoom {
                    center_x: x,
                    center_y: y,
                    scale,
                }));
            }
        },
        UseEventListenerOptions::default().passive(true),
    );

    //Zoom on pinch
    _ = use_event_listener_with_options(
        viewbox_ref,
        touchstart,
        move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                viewbox_state.is_panning.update_untracked(|b| *b = false);
                initial_touch_distance
                    .update(move |v| *v = touch_distance_and_center(viewbox_ref, &evt).0);
            }
        },
        UseEventListenerOptions::default().passive(true),
    );

    _ = use_event_listener_with_options(
        viewbox_ref,
        touchmove,
        move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                let (current_distance, center) = touch_distance_and_center(viewbox_ref, &evt);
                let scale = current_distance / initial_touch_distance();
                queue_update.with_value(|f| f(ViewBoxUpdateType::Zoom {
                    center_x: center.0,
                    center_y: center.1,
                    scale,
                }));
            }
        },
        UseEventListenerOptions::default().passive(true),
    );

    //Stop panning when user releases touch/click
    _ = use_event_listener(viewbox_ref, pointerup, move |_| {
        viewbox_state.is_panning.update_untracked(|b| *b = false);
    });

    //Stop panning when pointer leaves board area
    _ = use_event_listener(viewbox_ref, pointerleave, move |_| {
        viewbox_state.is_panning.update_untracked(|b| *b = false);
    });

    //Prevent right click/context menu on board
    _ = use_event_listener(viewbox_ref, contextmenu, move |evt| {
        evt.prevent_default();
    });

    _ = on_click_outside(g_ref, move |event| {
        let clicked_timer = event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("#timer").ok().flatten())
            .is_some();
        if !clicked_timer {
            game_state.reset();
        }
    });
    view! {
        <div node_ref=div_ref class=board_style style=background_style>

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

fn touch_distance_and_center(svg: NodeRef<svg::Svg>, evt: &TouchEvent) -> (f32, (f32, f32)) {
    let touches = evt.touches();
    let touch_0 = touches.get(0).expect("Should have first touch");
    let touch_1 = touches.get(1).expect("Should have second touch");
    
    let point_0 = screen_to_svg_coordinates(svg, touch_0.client_x() as f32, touch_0.client_y() as f32);
    let point_1 = screen_to_svg_coordinates(svg, touch_1.client_x() as f32, touch_1.client_y() as f32);
    
    let distance_x = point_0.0 - point_1.0;
    let distance_y = point_0.1 - point_1.1;
    let distance = distance_x.hypot(distance_y);
    
    let center = ((point_0.0 + point_1.0) / 2.0, (point_0.1 + point_1.1) / 2.0);
    
    (distance, center)
}

fn screen_to_svg_coordinates(svg: NodeRef<svg::Svg>, x: f32, y: f32) -> (f32, f32) {
    let svg = svg.get_untracked().expect("svg should exist already");
    let svg_graphics_element = svg.unchecked_ref::<web_sys::SvgGraphicsElement>();
    let svg_svg_element = svg.unchecked_ref::<web_sys::SvgsvgElement>();
    let point = svg_svg_element.create_svg_point();
    point.set_x(x);
    point.set_y(y);
    let transformed_point = point.matrix_transform(
        &svg_graphics_element
            .get_screen_ctm()
            .expect("screen ctm missing")
            .inverse()
            .expect("matrix not inversed"),
    );
    (transformed_point.x(), transformed_point.y())
}

fn will_svg_be_visible(g_ref: NodeRef<svg::G>, viewbox: &ViewBoxControls) -> bool {
    let bbox = g_ref
        .get_untracked()
        .expect("G exists")
        .unchecked_ref::<web_sys::SvgGraphicsElement>()
        .get_b_box()
        .expect("Rect");
    
    let bbox_mid_x = bbox.x() + viewbox.x_transform + bbox.width() / 2.0;
    let bbox_mid_y = bbox.y() + viewbox.y_transform + bbox.height() / 2.0;
    let viewbox_right = viewbox.x + viewbox.width;
    let viewbox_bottom = viewbox.y + viewbox.height;

    (bbox_mid_x > viewbox.x)
        && (bbox_mid_x < viewbox_right)
        && (bbox_mid_y > viewbox.y)
        && (bbox_mid_y < viewbox_bottom)
}
