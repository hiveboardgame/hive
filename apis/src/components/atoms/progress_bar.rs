use leptix_primitives::progress::{ProgressIndicator, ProgressRoot};
use leptos::*;

#[component]
pub fn ProgressBar(current: Signal<usize>, total: usize) -> impl IntoView {
    let progress = Signal::derive(move || {
        let progress = current.get() as f64 / total as f64;
        progress * 100.0
    });
    let indicator_style =
        Signal::derive(move || format!("transform: translateX(-{}%)", 100.0 - progress.get()));
    view! {
        <Show when=move || { total > 0 }>
            <div class="flex flex-col gap-1 justify-center items-center w-full">
                <div class="flex gap-1">
                    <span class="font-bold text-md">Games played:</span>
                    {current}
                    /
                    {total}
                </div>
                <div class="w-4/5">
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
            </div>
        </Show>
    }
}
