use leptos::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    username: String,
) -> impl IntoView {
    let profile_link = format!("/@/{}", username);
    view! {
        <a
            class=format!(
                "z-20 relative font-bold duration-300 hover:text-blue-600 {extend_tw_classes}",
            )

            href=profile_link
        >
            <p class="whitespace-nowrap truncate max-w-[80px] lg:max-w-[120px]">{username}</p>
        </a>
    }
}
