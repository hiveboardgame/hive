use leptos::*;

#[component]
pub fn Home(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <div class=format!("{extend_tw_classes}")>
        Welcome to our Hive
        </div>
    }
}
