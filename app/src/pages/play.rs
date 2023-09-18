use crate::organisms::header::Header;
use crate::organisms::{board::Board, overlay_container::OverlayTabs};
use leptos::*;

#[component]
pub fn PlayPage(cx: Scope) -> impl IntoView {
    view! { cx,
        <div class="h-screen w-screen overflow-hidden">
            <Header/>
            <div class="grid grid-cols-10  items-stretch">
                <Board/>
                <div class="col-start-9 col-span-2 border-2 border-blue-200 h-3/4 mt-20">
                    <OverlayTabs/>
                </div>
            </div>

        </div>
    }
}
