use leptos::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    username: String,
) -> impl IntoView {
    let profile_link = format!("/@/{}", username);
    view! {
        <a
            class="z-20 relative font-bold duration-300 hover:text-blue-600 pt-[2px]"

            href=profile_link
        >
            <div class=format!("{extend_tw_classes}")>{username}</div>
        </a>
    }
}
