use crate::providers::annotations::{
    AnnotationColor,
    AnnotationTool,
    AnnotationsSignal,
    MarkerShape,
};
use leptos::prelude::*;

const BTN: &str = "flex justify-center items-center w-7 h-7 rounded transition-transform duration-200 active:scale-95 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal";

/// Pencil toggle for sticky annotate mode; lives in the control row. Renders
/// nothing without an `AnnotationsSignal` in context.
#[component]
pub fn AnnotationToggle(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let annotations = use_context::<AnnotationsSignal>();
    view! {
        {annotations
            .map(|annotations| {
                let mode = annotations.mode;
                view! {
                    // Match DownloadPgn's box so it lines up with the control icons.
                    <button
                        title="Annotate (markers, arrows, highlights)"
                        class=move || {
                            format!(
                                "flex z-20 justify-center items-center m-1 text-white rounded-sm transition-transform duration-300 active:scale-95 aspect-square dark:hover:bg-pillbug-teal hover:bg-pillbug-teal {extend_tw_classes} {}",
                                if mode.get() {
                                    "bg-pillbug-teal"
                                } else {
                                    "bg-button-dawn dark:bg-button-twilight"
                                },
                            )
                        }
                        on:click=move |_| annotations.toggle_mode()
                    >
                        <span class="flex justify-center items-center py-1 text-xl leading-none size-7">
                            "✎"
                        </span>
                    </button>
                }
            })}
    }
}

/// Floating color/tool palette, shown while drawing. (Toggle lives in the
/// control row — see [`AnnotationToggle`].)
#[component]
pub fn AnnotationToolbar(annotations: AnnotationsSignal) -> impl IntoView {
    let mode = annotations.mode;
    let quick_draw = annotations.quick_draw;
    view! {
        <Show when=move || mode.get() || quick_draw.get()>
            <div class="flex absolute bottom-2 left-1/2 z-20 gap-1 items-center p-1 rounded-lg shadow -translate-x-1/2 select-none bg-board-dawn/90 dark:bg-reserve-twilight/90">
                {AnnotationColor::all()
                    .into_iter()
                    .map(|color| color_swatch(annotations, color))
                    .collect_view()} <div class="mx-1 w-px h-5 bg-gray-400"></div>
                {tool_button(
                    annotations,
                    AnnotationTool::Highlight,
                    "⬡",
                    "Hexagon (Q)",
                    "text-3xl -translate-y-[2px]",
                )}
                {MarkerShape::all()
                    .into_iter()
                    .map(|shape| {
                        tool_button(
                            annotations,
                            AnnotationTool::Marker(shape),
                            shape_glyph(shape),
                            shape_title(shape),
                            "text-base",
                        )
                    })
                    .collect_view()} <div class="mx-1 w-px h-5 bg-gray-400"></div>
                <button
                    title="Clear annotations on this position"
                    class=format!("{BTN} bg-inherit")
                    on:click=move |_| annotations.clear_current()
                >
                    "🗑"
                </button>
            </div>
        </Show>
    }
}

/// Modifier held while drawing to pick this color.
fn color_hint(color: AnnotationColor) -> &'static str {
    match color {
        AnnotationColor::White => "White (Ctrl)",
        AnnotationColor::Black => "Black (Alt)",
        AnnotationColor::Red => "Red (Ctrl+Alt)",
        AnnotationColor::Green => "Green (Meta)",
    }
}

fn color_swatch(annotations: AnnotationsSignal, color: AnnotationColor) -> impl IntoView {
    let selected = annotations.color;
    // Neutral border so the white/black swatches stay visible on either bg.
    let style = format!("background-color: {}", color.fill());
    view! {
        <button
            title=color_hint(color)
            class=move || {
                format!(
                    "w-6 h-6 rounded-full border border-gray-500 transition-transform duration-200 outline-2 {}",
                    if selected.get() == color {
                        "scale-110 outline outline-black dark:outline-white"
                    } else {
                        "outline-none"
                    },
                )
            }
            style=style
            on:click=move |_| selected.set(color)
        ></button>
    }
}

fn tool_button(
    annotations: AnnotationsSignal,
    tool: AnnotationTool,
    glyph: &'static str,
    title: &'static str,
    glyph_class: &'static str,
) -> impl IntoView {
    let selected = annotations.tool;
    view! {
        <button
            title=title
            class=move || {
                format!(
                    "{BTN} {}",
                    if selected.get() == tool { "bg-pillbug-teal" } else { "bg-inherit" },
                )
            }
            on:click=move |_| selected.set(tool)
        >
            <span class=format!("inline-block leading-none {glyph_class}")>{glyph}</span>
        </button>
    }
}

fn shape_glyph(shape: MarkerShape) -> &'static str {
    match shape {
        MarkerShape::Circle => "●",
        MarkerShape::Cross => "✕",
    }
}

fn shape_title(shape: MarkerShape) -> &'static str {
    match shape {
        MarkerShape::Circle => "Circle (W)",
        MarkerShape::Cross => "Cross (E)",
    }
}
