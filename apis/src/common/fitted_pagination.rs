use leptos::{
    ev::resize,
    html::Div,
    leptos_dom::helpers::{request_animation_frame_with_handle, AnimationFrameRequestHandle},
    prelude::*,
};
use leptos_use::{use_event_listener, use_resize_observer, use_window};
use wasm_bindgen::JsCast;
use web_sys::Element;

pub(crate) struct FittedPagination {
    pub page: RwSignal<usize>,
    pub page_size: RwSignal<usize>,
}

// The pager is 40px tall with 12px above it. Leave room for panel/page padding too.
const PAGER_SPACE: f64 = 52.0;
const BOTTOM_SPACE: f64 = 32.0;

fn fitted_page_size(root: &Element, total: usize) -> Option<usize> {
    let window = web_sys::window()?;
    let items = root.query_selector("[data-page-items]").ok()??;
    let rows = items.query_selector_all("[data-page-row]").ok()?;
    let first = rows.item(0)?.dyn_into::<Element>().ok()?;
    let first_bounds = first.get_bounding_client_rect();
    let row_height = (0..rows.length())
        .filter_map(|index| rows.item(index)?.dyn_into::<Element>().ok())
        .map(|row| row.get_bounding_client_rect().height())
        .fold(0.0_f64, f64::max);
    if row_height <= 0.0 {
        return None;
    }
    let style = window.get_computed_style(&items).ok()??;
    let columns = style.get_property_value("grid-template-columns").ok()?;
    let columns = columns.split_whitespace().count().max(1);
    let gap = style
        .get_property_value("row-gap")
        .ok()?
        .trim_end_matches("px")
        .parse::<f64>()
        .unwrap_or(0.0);
    // Document coordinates keep page capacity stable when the user scrolls.
    let top = first_bounds.top() + window.scroll_y().ok()?;
    let available = window.inner_height().ok()?.as_f64()? - top - BOTTOM_SPACE;
    let capacity = |space: f64| {
        (((space + gap) / (row_height + gap)).floor().max(1.0) as usize).saturating_mul(columns)
    };
    let without_pager = capacity(available);
    Some(if total <= without_pager {
        without_pager
    } else {
        capacity(available - PAGER_SPACE)
    })
}

/// Sizes uniform table/grid rows to the viewport. Callers mark the row container
/// and rows with data-page-items/data-page-row and place a 52px pager below them.
pub(crate) fn use_fitted_pagination(
    container: NodeRef<Div>,
    total: Signal<usize>,
    initial_page_size: usize,
) -> FittedPagination {
    let page = ArcRwSignal::new(1usize);
    let page_size = ArcRwSignal::new(initial_page_size.max(1));
    let mounted = ArcRwSignal::new(true);
    let pending = ArcStoredValue::new(None::<AnimationFrameRequestHandle>);
    let cleanup_mounted = mounted.clone();
    let cleanup_pending = pending.clone();
    on_cleanup(move || {
        cleanup_mounted.set(false);
        if let Some(handle) = cleanup_pending.get_value() {
            handle.cancel();
        }
    });

    let measure = Callback::new({
        let mounted = mounted.clone();
        let page = page.clone();
        let page_size = page_size.clone();
        move |()| {
            if !mounted.get_untracked() {
                return;
            }
            if let Some(handle) = pending.get_value() {
                handle.cancel();
            }
            let Some(root) = container.get_untracked() else {
                return;
            };
            let total = total.get_untracked();
            let mounted = mounted.clone();
            let pending_frame = pending.clone();
            let page = page.clone();
            let page_size = page_size.clone();
            // Only the mounted DOM element and Arc-backed state cross the frame boundary.
            let handle = request_animation_frame_with_handle(move || {
                pending_frame.set_value(None);
                if !mounted.get_untracked() || !root.is_connected() {
                    return;
                }
                let Some(next_size) = fitted_page_size(&root, total) else {
                    return;
                };
                let previous_size = page_size.get_untracked();
                if next_size != previous_size {
                    let first = page.get_untracked().saturating_sub(1) * previous_size;
                    batch(move || {
                        page_size.set(next_size);
                        page.set((first / next_size + 1).min(total.div_ceil(next_size).max(1)));
                    });
                }
            });
            pending.set_value(handle.ok());
        }
    });
    let observer_mounted = mounted.clone();
    use_resize_observer(container, move |_, _| {
        if observer_mounted.get_untracked() {
            measure.run(());
        }
    });
    let _ = use_event_listener(use_window(), resize, move |_| {
        if mounted.get_untracked() {
            measure.run(());
        }
    });
    Effect::new(move |_| {
        total.track();
        container.track();
        measure.run(());
    });
    let page = RwSignal::from(page);
    let page_size = RwSignal::from(page_size);
    Effect::new(move |_| {
        let last = total.get().div_ceil(page_size.get()).max(1);
        let current = page.get_untracked();
        if current == 0 || current > last {
            page.set(current.clamp(1, last));
        }
    });
    FittedPagination { page, page_size }
}
