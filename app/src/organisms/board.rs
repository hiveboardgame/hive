use crate::atoms::svgs::Svgs;
use crate::molecules::board_pieces::BoardPieces;
use ::wasm_bindgen::JsCast;
use ::web_sys::PointerEvent;
use leptos::ev::{contextmenu, pointerdown, pointerleave, pointermove, pointerup};
use leptos::svg::Svg;
use leptos::*;
use leptos_use::use_event_listener;

#[derive(Debug, Clone)]
struct ViewBoxControls {
    x: f32,
    y: f32,
    height: f32,
    width: f32,
    drag_start_x: f32,
    drag_start_y: f32,
}

impl ViewBoxControls {
    pub fn new() -> Self {
        ViewBoxControls {
            x: 1160.0,
            y: 525.0,
            height: 550.0,
            width: 550.0,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
        }
    }
}

#[component]
pub fn Board(cx: Scope) -> impl IntoView {
    let is_panning = create_rw_signal(cx, false);
    let viewbox_signal = create_rw_signal(cx, ViewBoxControls::new());
    let viewbox_ref = create_node_ref::<svg::Svg>(cx);

    let view_box_string = move || {
        format!(
            "{} {} {} {}",
            viewbox_signal().x,
            viewbox_signal().y,
            viewbox_signal().width,
            viewbox_signal().height
        )
    };

    create_effect(cx, move |_| {
        _ = use_event_listener(cx, viewbox_ref, pointerdown, move |evt| {
            is_panning.update_untracked(|b| *b = true);
            let ref_point = svg_point_from_event(viewbox_ref, evt);
            viewbox_signal.update(|view_box_controls: &mut ViewBoxControls| {
                view_box_controls.drag_start_x = ref_point.x();
                view_box_controls.drag_start_y = ref_point.y();
            });
        });

        _ = use_event_listener(cx, viewbox_ref, pointermove, move |evt| {
            if is_panning.get_untracked() {
                let moved_point = svg_point_from_event(viewbox_ref, evt);
                viewbox_signal.update(|view_box_controls: &mut ViewBoxControls| {
                    view_box_controls.x -= moved_point.x() - view_box_controls.drag_start_x;
                    view_box_controls.y -= moved_point.y() - view_box_controls.drag_start_y;
                });
            }
        });

        _ = use_event_listener(cx, viewbox_ref, pointerup, move |_| {
            is_panning.update_untracked(|b| *b = false);
        });

        _ = use_event_listener(cx, viewbox_ref, pointerleave, move |_| {
            is_panning.update_untracked(|b| *b = false);
        });

        _ = use_event_listener(cx, viewbox_ref, contextmenu, move |evt| {
            evt.prevent_default();
        });
    });

    view! { cx,
        <svg
            viewBox=view_box_string
            class="touch-none h-screen w-screen"
            ref=viewbox_ref
            xmlns="http://www.w3.org/2000/svg"
        >
            <Svgs/>
            <BoardPieces/>
        </svg>
    }
}

fn svg_point_from_event(svg: NodeRef<Svg>, evt: PointerEvent) -> web_sys::SvgPoint {
    let svg = svg.get_untracked().expect("svg should exist already");
    let svg_graphics_element = svg.unchecked_ref::<web_sys::SvgGraphicsElement>();
    let svg_svg_element = svg.unchecked_ref::<web_sys::SvgsvgElement>();
    let point: web_sys::SvgPoint = svg_svg_element.create_svg_point();
    point.set_x(evt.client_x() as f32);
    point.set_y(evt.client_y() as f32);

    point.matrix_transform(
        &svg_graphics_element
            .get_screen_ctm()
            .expect("screen ctm missing")
            .inverse()
            .expect("matrix not inversed"),
    )
}
