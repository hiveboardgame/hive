use crate::common::{render_text_prop, with_class};
#[cfg(not(feature = "ssr"))]
use crate::components::layouts::navigation_focus::NavigationFocusState;
#[cfg(not(feature = "ssr"))]
use leptos::leptos_dom::helpers::request_animation_frame;
use leptos::{html, prelude::*};

#[component]
pub fn PageHeader(
    #[prop(into)] title: TextProp,
    #[prop(optional, into)] subtitle: Option<TextProp>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let has_subtitle = subtitle.is_some();
    let subtitle = subtitle.unwrap_or_default();
    let heading_ref = NodeRef::<html::H1>::new();

    #[cfg(not(feature = "ssr"))]
    if let Some(focus_state) = use_context::<NavigationFocusState>() {
        let mounted = ArcRwSignal::new(true);
        on_cleanup({
            let mounted = mounted.clone();
            move || mounted.set(false)
        });
        Effect::new(move |_| {
            let Some(request) = focus_state.pending_focus() else {
                return;
            };
            let Some(heading) = heading_ref.get() else {
                return;
            };
            let mounted = mounted.clone();
            let focus_state = focus_state.clone();
            request_animation_frame(move || {
                if !mounted.get_untracked() || !focus_state.focus_request_is_current(&request) {
                    return;
                }
                request_animation_frame(move || {
                    if !mounted.get_untracked() || !focus_state.focus_request_is_current(&request) {
                        return;
                    }
                    if focus_state.consume_focus(&request) {
                        let _ = heading.focus();
                    }
                });
            });
        });
    }

    view! {
        <header class=with_class("flex flex-col gap-1", class.unwrap_or_default())>
            <h1
                node_ref=heading_ref
                tabindex="-1"
                data-navigation-focus-managed=""
                class="ui-page-title"
            >
                {render_text_prop(title)}
            </h1>
            <Show when=move || has_subtitle>
                <p class="ui-page-subtitle">{render_text_prop(subtitle.clone())}</p>
            </Show>
        </header>
    }
}
