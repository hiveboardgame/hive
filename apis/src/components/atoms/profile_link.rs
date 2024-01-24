use leptos::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    username: String,
) -> impl IntoView {
    let profile_link = format!("/@/{}", username);
    view! {
        <a
            class="z-20 relative font-bold duration-300 hover:text-pillbug-teal"

            href=profile_link
        >
            <div class=extend_tw_classes>{username}</div>
        </a>
    }
}
