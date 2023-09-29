use crate::components::organisms::header::Header;
use crate::components::organisms::{board::Board, side_board::SideboardTabs};
use leptos::*;

#[component]
pub fn PlayPage(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <div class=format!("{extend_tw_classes}")>
            <Header/>
            <div class="grid grid-cols-10 grid-rows-6 h-full w-full">
                <Board/>
                <SideboardTabs extend_tw_classes="border-blue-200"/>
            </div>

        </div>
    }
}
