use crate::{common::with_class, hooks::site_footer::use_site_footer};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub enum PageShellVariant {
    Content,
    Dashboard,
    Tournament,
    Form,
}

impl PageShellVariant {
    fn class(self) -> &'static str {
        match self {
            PageShellVariant::Content => {
                "mx-auto flex w-full max-w-5xl flex-col gap-3 px-3 pb-10 pt-page-gutter min-[360px]:px-4 sm:gap-6 sm:px-6"
            }
            PageShellVariant::Dashboard => {
                "flex w-full flex-col items-center justify-start gap-3 px-1.5 pb-8 pt-page-gutter min-[360px]:px-2 sm:gap-6 sm:px-4"
            }
            PageShellVariant::Tournament => {
                "flex w-full flex-col items-center justify-start gap-3 px-1.5 pb-8 pt-page-gutter min-[360px]:px-2 sm:px-[1vw]"
            }
            PageShellVariant::Form => {
                "mx-auto flex w-full max-w-3xl flex-1 flex-col justify-center px-3 py-14 min-[360px]:px-4 sm:px-6 sm:py-20"
            }
        }
    }

    pub(crate) fn footer_class(self) -> &'static str {
        match self {
            PageShellVariant::Content | PageShellVariant::Form => "max-w-5xl px-4 sm:px-6",
            // .ui-home-grid's max widths plus the 1rem shell padding on each side.
            PageShellVariant::Dashboard | PageShellVariant::Tournament => {
                "max-w-[58rem] px-4 lg:max-w-[82rem] 2xl:max-w-[108rem]"
            }
        }
    }
}

#[component]
pub fn PageShell(
    children: Children,
    #[prop(default = PageShellVariant::Content)] variant: PageShellVariant,
    #[prop(optional, into)] class: Option<String>,
    #[prop(default = true)] footer: bool,
) -> impl IntoView {
    if footer {
        use_site_footer(variant.footer_class());
    }
    view! { <div class=with_class(variant.class(), class.unwrap_or_default())>{children()}</div> }
}
