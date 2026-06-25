use crate::common::with_class;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub enum PageShellVariant {
    Content,
    Dashboard,
    Form,
}

impl PageShellVariant {
    fn class(self) -> &'static str {
        match self {
            PageShellVariant::Content => {
                "mx-auto flex w-full max-w-5xl flex-col gap-6 px-3 pb-10 pt-14 min-[360px]:px-4 sm:px-6 sm:pt-20"
            }
            PageShellVariant::Dashboard => {
                "flex w-full flex-col items-center justify-start gap-6 px-1.5 pb-8 pt-14 min-[360px]:px-2 sm:px-4 sm:pt-20"
            }
            PageShellVariant::Form => {
                "mx-auto flex min-h-screen w-full max-w-3xl flex-col justify-center px-3 py-14 min-[360px]:px-4 sm:px-6 sm:py-20"
            }
        }
    }
}

#[component]
pub fn PageShell(
    children: Children,
    #[prop(default = PageShellVariant::Content)] variant: PageShellVariant,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    view! { <div class=with_class(variant.class(), class.unwrap_or_default())>{children()}</div> }
}
