use leptos::prelude::*;
use markdown::{CompileOptions, Options};

#[component]
pub fn RlBanner(title: String, content: String) -> impl IntoView {
    //Unrestricted markdown, allows images

    let markdown_desc = move || {
        markdown::to_html_with_options(
            &content,
            &Options {
                compile: CompileOptions {
                    allow_dangerous_html: true,
                    allow_dangerous_protocol: false,
                    ..CompileOptions::default()
                },
                ..Options::default()
            },
        )
        .unwrap()
    };
    view! {
        <div class="overflow-hidden relative mb-3 text-black rounded-xl ring-1 shadow-sm dark:text-white ring-black/5 bg-even-light xs:mb-4 dark:ring-white/10 dark:bg-even-dark">
            <div class="absolute inset-y-0 left-0 w-1.5 bg-gradient-to-b from-pillbug-teal to-button-dawn" />
            <div class="py-6 pr-6 pl-8 xs:py-8 xs:pr-8 xs:pl-10">
                <h1 class="mb-3 text-2xl font-bold xs:text-4xl">{title}</h1>
                <div class="max-w-none prose dark:prose-invert" inner_html=markdown_desc />
            </div>
        </div>
    }
}
