use leptos::prelude::*;

#[component]
pub fn ProgressBar(current: Signal<usize>, total: usize) -> impl IntoView {
    let indicator_style = Signal::derive(move || {
        format!("width: {}%", {
            let progress = (current() as f64 / total as f64).clamp(0.0, 1.0);
            progress * 100.0
        })
    });
    view! {
        <Show when=move || { total > 0 }>
            <div class="flex flex-col gap-1 justify-center items-center w-full">
                <div class="flex gap-1">
                    <span class="font-bold text-md">Games played:</span>
                    {current}
                    /
                    {total}
                </div>
                <div class="overflow-hidden w-4/5 h-5 rounded-full border shadow-sm border-black/10 bg-odd-light dark:border-white/10 dark:bg-[#171d25]">
                    <div class="h-full rounded-full bg-orange-twilight" style=indicator_style></div>
                </div>
            </div>
        </Show>
    }
}
