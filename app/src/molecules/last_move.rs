
use leptos::*;

#[component]
fn LastMove(cx: Scope) -> impl IntoView {
    view! { cx,
        <g class="lastmove">
            <g id="lastmove">
                <use_ href="#lastmove" transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
            </g>
        </g>
    }
}
