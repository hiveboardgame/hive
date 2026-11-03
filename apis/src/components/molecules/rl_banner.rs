use crate::components::{atoms::local_date_time::LocalDateTime, molecules::page_card::PageCard};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use markdown::{CompileOptions, Options};

#[component]
pub fn RlBanner(title: String, content: String, starts_at: Option<DateTime<Utc>>) -> impl IntoView {
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
        <PageCard class="overflow-hidden relative mb-3 text-black dark:text-white xs:mb-4">
            <div class="absolute inset-y-0 left-0 w-1.5 bg-gradient-to-b from-pillbug-teal to-button-dawn" />
            <div class="py-6 pr-6 pl-8 xs:py-8 xs:pr-8 xs:pl-10">
                <h1 class="mb-3 text-2xl font-bold xs:text-4xl">{title}</h1>
                {starts_at
                    .map(|date_time| {
                        view! {
                            <p class="flex gap-2 items-center mb-3 text-lg font-semibold">
                                <Icon
                                    icon=icondata_bs::BsCalendarEvent
                                    attr:class="shrink-0 size-5"
                                />
                                <LocalDateTime date_time />
                            </p>
                        }
                    })}
                <div class="max-w-none prose dark:prose-invert" inner_html=markdown_desc />
            </div>
        </PageCard>
    }
}
