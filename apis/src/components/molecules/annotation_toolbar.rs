use crate::{
    i18n::*,
    providers::annotations::{AnnotationColor, AnnotationTool, AnnotationsSignal, MarkerShape},
};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use leptos_icons::*;

const BTN: &str = "flex justify-center items-center size-8 overflow-hidden rounded-md transition-colors duration-200 active:scale-95 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal";
const TOOLBAR_DIVIDER_CLASS: &str = "mx-1 w-px h-5 bg-black/20 dark:bg-white/20";

/// Pencil toggle for sticky annotate mode; lives in the control row. Renders
/// nothing without an `AnnotationsSignal` in context.
#[component]
pub fn AnnotationToggle(
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] active_tw_classes: Option<&'static str>,
) -> impl IntoView {
    let i18n = use_i18n();
    let annotations = use_context::<AnnotationsSignal>();
    let uses_default_button = class.is_none();
    let base_class = class.unwrap_or("ui-button ui-button-icon m-1");
    let active_tw_classes = active_tw_classes.unwrap_or("ui-button-primary");
    let inactive_tw_classes = if uses_default_button {
        "ui-button-secondary"
    } else {
        ""
    };
    view! {
        {annotations
            .map(|annotations| {
                let mode = annotations.mode;
                let button_class = move || {
                    format!(
                        "{} {}",
                        base_class,
                        if mode.get() { active_tw_classes } else { inactive_tw_classes },
                    )
                };
                view! {
                    // Match DownloadPgn's box so it lines up with the control icons.
                    <button
                        title=move || t_string!(i18n, analysis.annotate_markers_arrows_highlights)
                        class=button_class
                        on:click=move |_| annotations.toggle_mode()
                    >
                        <Icon icon=icondata_bs::BsPencil attr:class="size-5" />
                    </button>
                }
            })}
    }
}

/// Floating color/tool palette, shown while drawing. (Toggle lives in the
/// control row — see [`AnnotationToggle`].)
#[component]
pub fn AnnotationToolbar(annotations: AnnotationsSignal) -> impl IntoView {
    let i18n = use_i18n();
    let mode = annotations.mode;
    let quick_draw = annotations.quick_draw;
    view! {
        <Show when=move || mode.get() || quick_draw.get()>
            <div class="flex absolute bottom-2 left-1/2 z-20 gap-1 items-center p-1 rounded-lg border ring-1 shadow-lg -translate-x-1/2 select-none border-black/10 bg-even-light/95 ring-black/5 backdrop-blur dark:border-white/10 dark:bg-surface-panel/95 dark:ring-white/10">
                <div class="flex gap-1 items-center">
                    {AnnotationColor::all()
                        .into_iter()
                        .map(|color| color_swatch(annotations, color))
                        .collect_view()}
                </div>
                <div class=TOOLBAR_DIVIDER_CLASS></div>
                {tool_button(
                    annotations,
                    AnnotationTool::Highlight,
                    "⬡",
                    move || t_string!(i18n, analysis.hexagon_q).to_string(),
                    "text-2xl",
                )}
                {MarkerShape::all()
                    .into_iter()
                    .map(|shape| {
                        tool_button(
                            annotations,
                            AnnotationTool::Marker(shape),
                            shape_glyph(shape),
                            move || shape_title(i18n, shape),
                            "text-base",
                        )
                    })
                    .collect_view()}
                <div class=TOOLBAR_DIVIDER_CLASS></div>
                <button
                    title=move || t_string!(i18n, analysis.clear_annotations_on_this_position)
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
fn color_hint(i18n: I18nContext<Locale, I18nKeys>, color: AnnotationColor) -> String {
    match color {
        AnnotationColor::White => t_string!(i18n, analysis.white_ctrl).to_string(),
        AnnotationColor::Black => t_string!(i18n, analysis.black_alt).to_string(),
        AnnotationColor::Red => t_string!(i18n, analysis.red_ctrl_alt).to_string(),
        AnnotationColor::Green => t_string!(i18n, analysis.green_meta).to_string(),
    }
}

fn color_swatch(annotations: AnnotationsSignal, color: AnnotationColor) -> impl IntoView {
    let i18n = use_i18n();
    let selected = annotations.color;
    // Neutral border so the white/black swatches stay visible on either bg.
    let style = format!("background-color: {}", color.fill());
    view! {
        <button
            title=move || color_hint(i18n, color)
            class="flex justify-center items-center size-7"
            on:click=move |_| selected.set(color)
        >
            <span
                class=move || {
                    format!(
                        "block size-5 rounded-full border border-black/30 transition-transform duration-200 outline-2 dark:border-white/30 {}",
                        if selected.get() == color {
                            "scale-110 outline outline-black dark:outline-white"
                        } else {
                            "outline-none"
                        },
                    )
                }
                style=style
            ></span>
        </button>
    }
}

fn tool_button(
    annotations: AnnotationsSignal,
    tool: AnnotationTool,
    glyph: &'static str,
    title: impl Fn() -> String + Send + Sync + Copy + 'static,
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

fn shape_title(i18n: I18nContext<Locale, I18nKeys>, shape: MarkerShape) -> String {
    match shape {
        MarkerShape::Circle => t_string!(i18n, analysis.circle_w).to_string(),
        MarkerShape::Cross => t_string!(i18n, analysis.cross_e).to_string(),
    }
}
