use crate::common::{render_text_prop, with_class};
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn Panel(
    children: Children,
    #[prop(optional, into)] title: Option<TextProp>,
    #[prop(optional)] icon: Option<icondata_core::Icon>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] body_class: Option<String>,
) -> impl IntoView {
    let has_title = title.is_some();
    let title = title.unwrap_or_default();

    view! {
        <section class=with_class("ui-panel", class.unwrap_or_default())>
            <Show when=move || has_title>
                <div class="ui-panel-header">
                    <div>
                        <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">
                            {icon
                                .map(|icon| {
                                    view! {
                                        <Icon
                                            icon=icon
                                            attr:class="inline-block mr-2 size-5 align-text-bottom"
                                            attr:aria-hidden="true"
                                        />
                                    }
                                })} {render_text_prop(title.clone())}
                        </h2>
                    </div>
                </div>
            </Show>
            <div class=with_class(
                "ui-panel-body",
                body_class.unwrap_or_default(),
            )>{children()}</div>
        </section>
    }
}
