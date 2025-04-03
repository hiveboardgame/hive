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
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">{title}</h1>
            <div class="prose dark:prose-invert" inner_html=markdown_desc />
        </div>
    }
}
