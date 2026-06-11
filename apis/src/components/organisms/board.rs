use crate::{
    common::{position_from_svg, SvgPos, TileDesign},
    components::{
        layouts::base_layout::OrientationSignal,
        molecules::{
            annotation_toolbar::AnnotationToolbar,
            annotations_layer::AnnotationsLayer,
            board_pieces::BoardPieces,
            history_pieces::HistoryPieces,
        },
    },
    hiveground::HivegroundInteraction,
    providers::{
        analysis::AnalysisSignal,
        annotations::{AnnotationColor, AnnotationTool, AnnotationsSignal, MarkerShape},
        game_state::{GameState, GameStateSignal, View},
        Config,
    },
};
use hive_lib::{GameStatus, Position, State};
use leptos::{
    either::Either,
    ev::{
        contextmenu,
        keydown,
        keyup,
        pointerdown,
        pointerenter,
        pointerleave,
        pointermove,
        pointerup,
        touchcancel,
        touchend,
        touchmove,
        touchstart,
        wheel,
    },
    html,
    prelude::*,
    svg,
};
use leptos_use::{
    on_click_outside,
    use_event_listener,
    use_event_listener_with_options,
    use_intersection_observer_with_options,
    use_raf_fn,
    use_resize_observer,
    use_timeout_fn,
    use_window,
    UseEventListenerOptions,
    UseIntersectionObserverOptions,
    UseTimeoutFnReturn,
};
use wasm_bindgen::JsCast;
use web_sys::{Element, EventTarget, KeyboardEvent, PointerEvent, TouchEvent, WheelEvent};

// Movement under this (client px) is a click, not a drag.
const ANNOTATION_TAP_SLOP_PX: f64 = 8.0;
const STACK_LONG_PRESS_DELAY_MS: f64 = 500.0;
const STACK_TOUCH_MOVE_CANCEL_THRESHOLD_PX: f64 = 8.0;
const ZOOM_WHEEL_SENSITIVITY: f32 = 0.002; // scroll-wheel: per-unit deltaY -> scale fraction
const ZOOM_PINCH_SENSITIVITY: f32 = 0.005; // trackpad pinch (ctrlKey wheel): faster than scroll
const ZOOM_WHEEL_MAX_STEP: f32 = 0.10; // cap one event at ~10% zoom

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
    // Pinch zoom is absolute: `scale` is the cumulative ratio since the gesture
    // started, applied to `base` (the viewbox snapshotted at touchstart). Because
    // every frame recomputes from the fixed snapshot, the RAF queue coalescing
    // away intermediate touchmove events is harmless — the latest event still
    // lands on the correct zoom.
    PinchZoom {
        base: ViewBoxControls,
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum StackExpansionResetKey {
    CurrentTurn {
        turn: usize,
        hash: Option<u64>,
    },
    HistoryTurn {
        turn: Option<usize>,
        hash: Option<u64>,
    },
}

#[component]
pub fn Board(interaction: HivegroundInteraction, history_state: Memo<State>) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let analysis = use_context::<AnalysisSignal>();
    let annotations = use_context::<AnnotationsSignal>();
    // Hex where the current draw gesture started (set on the draw-button press).
    let annotation_start = RwSignal::new(None::<Position>);
    // Left-press client point, so a click (no drag) while drawing can clear all.
    let left_press = RwSignal::new(None::<(i32, i32)>);
    let orientation_signal = expect_context::<OrientationSignal>();
    let config = expect_context::<Config>().0;
    let viewbox_state = ViewBoxState::new();
    let viewbox_signal = RwSignal::new(ViewBoxControls::new());
    let initial_touch_distance = RwSignal::<f32>::new(0.0);
    // Snapshot taken at the start of a pinch: (viewbox, anchor_x, anchor_y).
    let pinch_base = RwSignal::<Option<(ViewBoxControls, f32, f32)>>::new(None);
    let pending_update = RwSignal::new(None::<ViewBoxUpdateType>);
    let viewbox_ref = NodeRef::<svg::Svg>::new();
    let g_ref = NodeRef::<svg::G>::new();
    let div_ref = NodeRef::<html::Div>::new();
    let last_turn = game_state.is_last_turn_as_signal();
    let board_view = create_read_slice(game_state.signal, |gs| gs.view.clone());
    let in_analysis = analysis.is_some();
    let stack_expansion_reset_key = create_read_slice(game_state.signal, move |gs| {
        stack_expansion_reset_key(gs, in_analysis)
    });
    let game_status = create_read_slice(game_state.signal, |gs| gs.state.game_status.clone());
    let board_style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex relative grow min-h-0"
        } else {
            "relative col-span-8 row-span-6"
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

    let current_center = game_state
        .signal
        .with_untracked(|gs| gs.state.board.center_coordinates());

    let straight = config.with_untracked(|c| c.tile.design == TileDesign::ThreeD);
    let tile_opts = Signal::derive(move || config.with(|c| c.tile.clone()));

    let background_style = Signal::derive(move || {
        let bg = config.with(|c| c.tile.get_effective_background_color(c.prefers_dark));
        format!("background-color: {bg}")
    });

    setup_stack_expansion_events(
        viewbox_ref,
        interaction,
        stack_expansion_reset_key,
        annotations,
    );

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
                    if viewbox_state.is_panning.get_untracked()
                        && interaction.is_viewport_pan_allowed()
                    {
                        let future_viewbox = viewbox_signal
                            .get_untracked()
                            .calculate_pan(delta_x, delta_y);
                        if viewbox_state.is_visible.get()
                            || will_svg_be_visible(g_ref, &future_viewbox)
                        {
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
                    let future_viewbox = viewbox_signal
                        .get_untracked()
                        .calculate_zoom(center_x, center_y, scale);
                    let intermediate_height = future_viewbox.height;

                    if (intermediate_height >= viewbox_state.zoom_in_limit
                        && intermediate_height <= viewbox_state.zoom_out_limit)
                        && (viewbox_state.is_visible.get()
                            || will_svg_be_visible(g_ref, &future_viewbox))
                    {
                        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
                            *viewbox_controls = future_viewbox;
                        });
                        viewbox_state.has_zoomed.set(true);
                    }
                }
                ViewBoxUpdateType::PinchZoom {
                    base,
                    center_x,
                    center_y,
                    scale,
                } => {
                    let future_viewbox = base.calculate_zoom(center_x, center_y, scale);
                    let intermediate_height = future_viewbox.height;

                    if (intermediate_height >= viewbox_state.zoom_in_limit
                        && intermediate_height <= viewbox_state.zoom_out_limit)
                        && (viewbox_state.is_visible.get()
                            || will_svg_be_visible(g_ref, &future_viewbox))
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
        queue_update.with_value(|f| {
            f(ViewBoxUpdateType::Resize {
                width: rect.width() as f32,
                height: rect.height() as f32,
            })
        });
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

    // Left-drag pans. Touch in armed mode draws instead (no right button), so it
    // doesn't pan; the mouse left-drag pans even while armed (right-drag draws).
    _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
        if evt.button() != 0 {
            return;
        }
        if is_touch_like(&evt) && is_drawing_event(annotations, &evt) {
            return;
        }
        viewbox_state.is_panning.update_untracked(|b| *b = true);
        let (x, y) = screen_to_svg_coordinates(viewbox_ref, evt.x() as f32, evt.y() as f32);
        viewbox_signal.update(|viewbox_controls: &mut ViewBoxControls| {
            viewbox_controls.drag_start_x = x;
            viewbox_controls.drag_start_y = y;
        });
        // A stationary left-click while drawing clears the position (checked on release).
        if annotation_drawing_on(annotations) {
            left_press.set(Some((evt.client_x(), evt.client_y())));
        }
    });

    // Sync quick-draw (palette + active color) to the modifiers held over the board.
    _ = use_event_listener(viewbox_ref, pointerenter, move |evt| {
        sync_quick_draw(annotations, &evt);
    });
    _ = use_event_listener(viewbox_ref, pointermove, move |evt| {
        sync_quick_draw(annotations, &evt);
    });

    // Pressing Ctrl/Alt/Meta opens the palette immediately (no pointer needed);
    // it closes as soon as the modifiers are released.
    if let Some(annotations) = annotations {
        _ = use_event_listener(use_window(), keydown, move |evt: KeyboardEvent| {
            if is_text_input_focused() {
                return;
            }
            apply_draw_modifiers(annotations, evt.ctrl_key(), evt.alt_key(), evt.meta_key());
            // While the palette is open, Q/W/E pick hexagon / circle / cross.
            if annotation_drawing_on(Some(annotations)) {
                match evt.key().as_str() {
                    "q" | "Q" => annotations.tool.set(AnnotationTool::Highlight),
                    "w" | "W" => annotations
                        .tool
                        .set(AnnotationTool::Marker(MarkerShape::Circle)),
                    "e" | "E" => annotations
                        .tool
                        .set(AnnotationTool::Marker(MarkerShape::Cross)),
                    _ => {}
                }
            }
        });
        _ = use_event_listener(use_window(), keyup, move |evt: KeyboardEvent| {
            apply_draw_modifiers(annotations, evt.ctrl_key(), evt.alt_key(), evt.meta_key());
        });
    }

    // The mouse draws with the right button, touch/pen with the primary press.
    // Release decides: different hex → arrow, same hex → mark.
    _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
        if let Some(annotations) = annotations {
            sync_quick_draw(Some(annotations), &evt);
            if is_draw_button(&evt) && is_drawing_event(Some(annotations), &evt) {
                evt.prevent_default();
                let position = pointer_hex(viewbox_ref, viewbox_signal, &evt);
                annotation_start.set(Some(position));
                // (from == to) previews the mark until the drag leaves the hex.
                annotations.preview.set(Some((position, position)));
            }
        }
    });

    // Track the live preview to the hex under the pointer during a drag.
    _ = use_event_listener(viewbox_ref, pointermove, move |evt| {
        if let Some(annotations) = annotations {
            if let Some(start) = annotation_start.get_untracked() {
                let end = pointer_hex(viewbox_ref, viewbox_signal, &evt);
                annotations.preview.set(Some((start, end)));
            }
        }
    });

    _ = use_event_listener(viewbox_ref, pointerup, move |evt| {
        if let Some(annotations) = annotations {
            if let Some(start) = annotation_start.get_untracked() {
                let point = pointer_point(viewbox_ref, viewbox_signal, &evt);
                let end = position_from_svg(point.0, point.1);
                if start != end {
                    annotations.apply_drag(start, end);
                } else {
                    annotations.apply_tap(end);
                }
                annotations.preview.set(None);
                annotation_start.set(None);
            }
        }
    });

    //Keep panning while user drags around
    _ = use_event_listener(viewbox_ref, pointermove, move |evt| {
        if viewbox_state.is_panning.get_untracked() && interaction.is_viewport_pan_allowed() {
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
                evt.prevent_default();
                let (x, y) = screen_to_svg_coordinates(viewbox_ref, evt.x() as f32, evt.y() as f32);
                let delta = evt.delta_y() as f32;
                // A trackpad pinch arrives as a wheel event with ctrlKey set; give it
                // its own (faster) sensitivity so it doesn't feel sluggish vs. scroll.
                let sensitivity = if evt.ctrl_key() {
                    ZOOM_PINCH_SENSITIVITY
                } else {
                    ZOOM_WHEEL_SENSITIVITY
                };
                let magnitude = (delta.abs() * sensitivity).min(ZOOM_WHEEL_MAX_STEP);
                let scale = if delta > 0.0 {
                    1.0 - magnitude
                } else {
                    1.0 + magnitude
                };
                queue_update.with_value(|f| {
                    f(ViewBoxUpdateType::Zoom {
                        center_x: x,
                        center_y: y,
                        scale,
                    })
                });
            }
        },
        UseEventListenerOptions::default().passive(false),
    );

    _ = use_event_listener_with_options(
        viewbox_ref,
        touchstart,
        move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                evt.prevent_default();
                viewbox_state.is_panning.update_untracked(|b| *b = false);
                // A second finger turns the gesture into zoom, so cancel any
                // mark/arrow started by the first touch.
                annotation_start.set(None);
                if let Some(annotations) = annotations {
                    annotations.preview.set(None);
                }
                // Snapshot the gesture start: finger spread (client px) plus the
                // current viewbox and the anchor point under the finger centroid.
                let (distance, center) = touch_distance_and_center(viewbox_ref, &evt);
                initial_touch_distance.set(distance);
                pinch_base.set(Some((viewbox_signal.get_untracked(), center.0, center.1)));
            }
        },
        UseEventListenerOptions::default().passive(false),
    );

    _ = use_event_listener_with_options(
        viewbox_ref,
        touchmove,
        move |evt: TouchEvent| {
            if evt.touches().length() == 2 {
                evt.prevent_default();
                let Some((base, center_x, center_y)) = pinch_base.get_untracked() else {
                    return;
                };
                let initial = initial_touch_distance.get_untracked();
                if initial <= 0.0 {
                    return;
                }
                // Cumulative ratio since gesture start; spreading fingers (current
                // > initial) yields scale > 1, which calculate_zoom maps to zoom-in.
                let (current_distance, _) = touch_distance_and_center(viewbox_ref, &evt);
                let scale = current_distance / initial;
                queue_update.with_value(|f| {
                    f(ViewBoxUpdateType::PinchZoom {
                        base: base.clone(),
                        center_x,
                        center_y,
                        scale,
                    })
                });
            }
        },
        UseEventListenerOptions::default().passive(false),
    );

    //Stop panning when user releases touch/click
    _ = use_event_listener(viewbox_ref, pointerup, move |evt| {
        viewbox_state.is_panning.update_untracked(|b| *b = false);
        // A stationary left-click while drawing clears this position's annotations.
        if let Some(annotations) = annotations {
            if evt.button() == 0 && !is_touch_like(&evt) && annotation_drawing_on(Some(annotations))
            {
                if let Some((px, py)) = left_press.get_untracked() {
                    let moved = ((evt.client_x() - px) as f64).hypot((evt.client_y() - py) as f64);
                    if moved <= ANNOTATION_TAP_SLOP_PX {
                        annotations.clear_current();
                    }
                }
            }
            left_press.set(None);
        }
    });

    //Stop panning when pointer leaves board area
    _ = use_event_listener(viewbox_ref, pointerleave, move |_| {
        viewbox_state.is_panning.update_untracked(|b| *b = false);
        // Leaving the board ends any in-progress drag. Quick-draw stays open while
        // the modifier is held (closed on keyup) so the palette can be clicked.
        if let Some(annotations) = annotations {
            annotations.preview.set(None);
        }
        annotation_start.set(None);
        left_press.set(None);
    });

    _ = on_click_outside(g_ref, move |event| {
        // While drawing, an off-hive click is an annotation, not a selection to cancel.
        if annotation_drawing_on(annotations) {
            return;
        }
        let clicked_timer = event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("#timer").ok().flatten())
            .is_some();
        if !clicked_timer {
            interaction.cancel_selection();
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
                <rect
                    x=move || viewbox_signal.with(|vb| vb.x)
                    y=move || viewbox_signal.with(|vb| vb.y)
                    width=move || viewbox_signal.with(|vb| vb.width)
                    height=move || viewbox_signal.with(|vb| vb.height)
                    fill="transparent"
                    pointer-events="all"
                />
                <g transform=transform node_ref=g_ref>
                    {move || {
                        if board_view() == View::History && !last_turn() && !in_analysis {
                            Either::Left(
                                view! { <HistoryPieces tile_opts interaction history_state /> },
                            )
                        } else {
                            Either::Right(view! { <BoardPieces tile_opts interaction /> })
                        }
                    }}
                    {annotations
                        .map(|annotations| {
                            view! { <AnnotationsLayer annotations history_state /> }
                        })}
                </g>
            </svg>
            {annotations.map(|annotations| view! { <AnnotationToolbar annotations /> })}
        </div>
    }
}

/// The quick-draw color for an event's held modifiers, if any (lichess-style).
fn draw_color_from_evt(evt: &PointerEvent) -> Option<AnnotationColor> {
    AnnotationColor::from_modifiers(evt.ctrl_key(), evt.alt_key(), evt.meta_key())
}

/// Touch and pen lack a right button and modifiers, so they draw with the
/// primary press; the mouse draws with the right button.
fn is_touch_like(evt: &PointerEvent) -> bool {
    evt.pointer_type() != "mouse"
}

/// The button that draws for this pointer: right for the mouse, primary otherwise.
fn is_draw_button(evt: &PointerEvent) -> bool {
    if is_touch_like(evt) {
        evt.button() == 0
    } else {
        evt.button() == 2
    }
}

/// Whether this event should draw (annotate mode or a held draw-modifier).
fn is_drawing_event(annotations: Option<AnnotationsSignal>, evt: &PointerEvent) -> bool {
    annotations.is_some_and(|annotations| {
        annotations.mode.get_untracked() || draw_color_from_evt(evt).is_some()
    })
}

/// Whether drawing is active (sticky mode or a held modifier).
fn annotation_drawing_on(annotations: Option<AnnotationsSignal>) -> bool {
    annotations.is_some_and(|annotations| {
        annotations.mode.get_untracked() || annotations.quick_draw.get_untracked()
    })
}

/// Reflect held modifiers into quick-draw state: open the palette and set the
/// active color, or clear it once the modifiers are released.
fn apply_draw_modifiers(annotations: AnnotationsSignal, ctrl: bool, alt: bool, meta: bool) {
    match AnnotationColor::from_modifiers(ctrl, alt, meta) {
        Some(color) => {
            if !annotations.quick_draw.get_untracked() {
                annotations.quick_draw.set(true);
            }
            if annotations.color.get_untracked() != color {
                annotations.color.set(color);
            }
        }
        None => {
            if annotations.quick_draw.get_untracked() {
                annotations.quick_draw.set(false);
            }
        }
    }
}

/// Sync quick-draw to the modifiers held during a pointer event.
fn sync_quick_draw(annotations: Option<AnnotationsSignal>, evt: &PointerEvent) {
    if let Some(annotations) = annotations {
        apply_draw_modifiers(annotations, evt.ctrl_key(), evt.alt_key(), evt.meta_key());
    }
}

/// Don't hijack Ctrl/Alt while the user is typing.
fn is_text_input_focused() -> bool {
    let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
    else {
        return false;
    };
    let tag = element.tag_name().to_lowercase();
    tag == "input" || tag == "textarea" || element.has_attribute("contenteditable")
}

/// Pointer position in the board's g-local space — undoes the pan/centering
/// translate on the `<g>` that `screen_to_svg_coordinates` doesn't account for.
fn pointer_point(
    svg: NodeRef<svg::Svg>,
    viewbox_signal: RwSignal<ViewBoxControls>,
    evt: &web_sys::PointerEvent,
) -> (f32, f32) {
    let (x, y) = screen_to_svg_coordinates(svg, evt.x() as f32, evt.y() as f32);
    let (tx, ty) = viewbox_signal.with_untracked(|vb| (vb.x_transform, vb.y_transform));
    (x - tx, y - ty)
}

/// The board hex a pointer event lands on.
fn pointer_hex(
    svg: NodeRef<svg::Svg>,
    viewbox_signal: RwSignal<ViewBoxControls>,
    evt: &web_sys::PointerEvent,
) -> Position {
    let (x, y) = pointer_point(svg, viewbox_signal, evt);
    position_from_svg(x, y)
}

fn setup_stack_expansion_events(
    viewbox_ref: NodeRef<svg::Svg>,
    interaction: HivegroundInteraction,
    reset_key: Signal<StackExpansionResetKey>,
    annotations: Option<AnnotationsSignal>,
) {
    let stack_touch_start = RwSignal::new(None::<(i32, i32)>);

    // Stack expansion stores only a position. Collapse it when the board content
    // behind that position changes, such as after a move or history navigation.
    Effect::watch(
        move || reset_key.get(),
        move |_, _, _| {
            interaction.collapse_stack();
        },
        false,
    );

    // Right-click inspects stacks only when drawing isn't armed; while armed the
    // right button draws instead.
    _ = use_event_listener(viewbox_ref, pointerdown, move |evt| {
        if evt.button() == 2 && !is_drawing_event(annotations, &evt) {
            if let Some(position) = stack_position_from_event_target(evt.target()) {
                evt.prevent_default();
                interaction.expand_stack(position);
            }
        }
    });

    let UseTimeoutFnReturn {
        start: start_stack_long_press,
        stop: stop_stack_long_press,
        ..
    } = use_timeout_fn(
        move |position: Position| {
            interaction.expand_stack(position);
        },
        STACK_LONG_PRESS_DELAY_MS,
    );
    let cancel_stack_long_press = StoredValue::new({
        let stop_stack_long_press = stop_stack_long_press.clone();
        move || {
            stack_touch_start.set(None);
            stop_stack_long_press();
        }
    });
    _ = use_event_listener_with_options(
        viewbox_ref,
        touchstart,
        move |evt: TouchEvent| match evt.touches().length() {
            // In edit mode a single finger draws, so don't long-press for stacks.
            1 if !annotations.is_some_and(|a| a.mode.get_untracked()) => {
                let Some(position) = stack_position_from_event_target(evt.target()) else {
                    cancel_stack_long_press.with_value(|cancel| cancel());
                    return;
                };
                let Some(touch) = evt.touches().get(0) else {
                    cancel_stack_long_press.with_value(|cancel| cancel());
                    return;
                };
                stack_touch_start.set(Some((touch.client_x(), touch.client_y())));
                start_stack_long_press(position);
            }
            _ => {
                cancel_stack_long_press.with_value(|cancel| cancel());
            }
        },
        UseEventListenerOptions::default().passive(true),
    );

    _ = use_event_listener_with_options(
        viewbox_ref,
        touchmove,
        move |evt: TouchEvent| match evt.touches().length() {
            1 => {
                let Some((start_x, start_y)) = stack_touch_start.get_untracked() else {
                    return;
                };
                let Some(touch) = evt.touches().get(0) else {
                    cancel_stack_long_press.with_value(|cancel| cancel());
                    return;
                };
                let delta_x = (touch.client_x() - start_x) as f64;
                let delta_y = (touch.client_y() - start_y) as f64;
                if delta_x.hypot(delta_y) > STACK_TOUCH_MOVE_CANCEL_THRESHOLD_PX {
                    cancel_stack_long_press.with_value(|cancel| cancel());
                }
            }
            _ => {
                cancel_stack_long_press.with_value(|cancel| cancel());
            }
        },
        UseEventListenerOptions::default().passive(true),
    );

    let window = use_window();
    _ = use_event_listener(window.clone(), pointerup, move |evt| {
        if evt.button() == 2 {
            interaction.collapse_stack();
        }
    });
    _ = use_event_listener_with_options(
        window.clone(),
        touchend,
        move |_| {
            cancel_stack_long_press.with_value(|cancel| cancel());
            interaction.collapse_stack();
        },
        UseEventListenerOptions::default().passive(true),
    );
    _ = use_event_listener_with_options(
        window,
        touchcancel,
        move |_| {
            cancel_stack_long_press.with_value(|cancel| cancel());
            interaction.collapse_stack();
        },
        UseEventListenerOptions::default().passive(true),
    );

    _ = use_event_listener(viewbox_ref, contextmenu, move |evt| {
        evt.prevent_default();
    });
}

fn stack_expansion_reset_key(game_state: &GameState, in_analysis: bool) -> StackExpansionResetKey {
    if game_state.view == View::History && !game_state.is_last_turn() && !in_analysis {
        let turn = game_state.history_turn;
        let hash = turn.and_then(|turn| game_state.state.history.hashes.get(turn).copied());
        return StackExpansionResetKey::HistoryTurn { turn, hash };
    }

    StackExpansionResetKey::CurrentTurn {
        turn: game_state.state.turn,
        hash: game_state.state.hashes.last().copied(),
    }
}

fn touch_distance_and_center(svg: NodeRef<svg::Svg>, evt: &TouchEvent) -> (f32, (f32, f32)) {
    let touches = evt.touches();
    let touch_0 = touches.get(0).expect("Should have first touch");
    let touch_1 = touches.get(1).expect("Should have second touch");

    // Distance in client pixels: a stable basis that does not shift as the viewBox
    // zooms, avoiding the compounding feedback loop of measuring in SVG coordinates.
    let dx = (touch_0.client_x() - touch_1.client_x()) as f32;
    let dy = (touch_0.client_y() - touch_1.client_y()) as f32;
    let distance = dx.hypot(dy);

    // Center stays in SVG coordinates: it is the zoom anchor for calculate_zoom.
    let point_0 =
        screen_to_svg_coordinates(svg, touch_0.client_x() as f32, touch_0.client_y() as f32);
    let point_1 =
        screen_to_svg_coordinates(svg, touch_1.client_x() as f32, touch_1.client_y() as f32);
    let center = ((point_0.0 + point_1.0) / 2.0, (point_0.1 + point_1.1) / 2.0);

    (distance, center)
}

fn stack_position_from_event_target(target: Option<EventTarget>) -> Option<Position> {
    target
        .and_then(|target| target.dyn_into::<Element>().ok())
        .and_then(stack_position_from_element)
}

fn stack_position_from_element(element: Element) -> Option<Position> {
    let stack = element
        .closest("[data-hg-stack-q][data-hg-stack-r]")
        .ok()
        .flatten()?;
    if stack.get_attribute("data-hg-stack-expandable").as_deref() != Some("true") {
        return None;
    }
    let q = stack.get_attribute("data-hg-stack-q")?.parse().ok()?;
    let r = stack.get_attribute("data-hg-stack-r")?.parse().ok()?;
    Some(Position::new(q, r))
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
