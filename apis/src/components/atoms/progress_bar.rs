use leptos::*;
use leptix_primitives::progress::{ProgressIndicator, ProgressRoot};

#[component]
pub fn ProgressBar(current: Signal<usize>, total: usize) -> impl IntoView {
    let progress = Signal::derive(move || {
        let progress = current.get() as f64 / total as f64;
        progress*100.0
    });
    let indicator_style = Signal::derive(move || format!("transform: translateX(-{}%)", 100.0 - progress.get()));
    view!{
        <div class="w-4/5">
        <span class="font-bold text-md">Progress:</span> {current} / {total}
        <ProgressRoot
        attr:class="relative overflow-hidden bg-white rounded-full w-full h-[20px] drop-shadow-md"
        attr:style="transform: translateZ(0)"
        value=progress
    >
        <ProgressIndicator
            attr:class="bg-orange-twilight w-full h-full transition-transform duration-[660ms] ease-[cubic-bezier(0.65, 0, 0.35, 1)]"
            attr:style=indicator_style
        />
    </ProgressRoot>
        </div>
    }

}
