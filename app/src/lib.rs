use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub mod atoms;
pub mod common;
pub mod error_template;
pub mod molecules;
pub mod organisms;

use crate::organisms::reserve::Reserve;
use hive_lib::color::Color;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);

    view! {
        cx,

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/start-actix-workspace.css"/>

        <meta name="viewport" content="width=device-width, initial-scale=1" />
        // sets the document title
        <Title text="Welcome to Hive"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    // Creates a reactive value to update the button
    //let (count, set_count) = create_signal(cx, 0);
    //let on_click = move |_| set_count.update(|count| *count += 1);
    let color = Color::White;

    view! { cx,
        <h1>"Navigation bar and banner goes here"</h1>
        // <button on:click=on_click>"Click Me: " {count}</button>
        <Reserve color/>
    }
}

#[component]
fn LastMove(cx: Scope) -> impl IntoView {
    view! { cx,
        <g class="lastmove">
            <g id="lastmove">
                <use_ href="#lastmove" transform="scale(0.56, 0.56) translate(-45, -50)" />
            </g>
        </g>
    }
}
