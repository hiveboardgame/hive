use crate::organisms::header::Header;
use crate::organisms::{board::Board, overlay_container::OverlayTabs};
use leptos::*;

#[component]
pub fn PlayPage(cx: Scope) -> impl IntoView {
    view! { cx,
        <div class="h-full w-full">
            <Header/>
            <div class="grid grid-cols-10 grid-rows-6 h-full w-full">
                <Board/>
                <div class="col-start-9 col-span-2 border-2 border-blue-200 row-span-4 row-start-2">
                    <OverlayTabs/>
                </div>
            </div>

        </div>
    }
}
