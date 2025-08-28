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
        <div class="px-4 w-full">
            <div class="mx-auto w-full max-w-4xl text-black bg-gradient-to-r rounded-lg ring-1 shadow-md from-orange-twilight to-orange-dawn ring-blue-dark/30">
                <div class="p-5 md:p-8">
                    <h1 class="mb-3 text-2xl font-bold tracking-tight md:text-4xl">{title}</h1>
                    <div class="prose prose-sm md:prose text-black/90 prose-a:underline prose-a:text-blue-dark" inner_html=markdown_desc />
                </div>
            </div>
        </div>
    }
}
