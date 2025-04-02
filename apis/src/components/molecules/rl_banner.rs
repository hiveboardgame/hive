use leptos::prelude::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">
                Hive World Cup Qualifier
            </h1>
            <h2 class="flex items-center mb-4 text-2xl font-bold xs:text-xl">Open to everyone!</h2>
            <p>
                <a
                    href="https://docs.google.com/document/d/1e_dVoYBEje6i1NNpEEiaQzRtyI1vZnaeDaN9GQ7-YLI/edit?tab=t.0"
                    rel="external"
                    target="_blank"
                    class="inline place-self-center text-blue-500 hover:underline"
                >
                    Read the rules
                </a>
            </p>
            <p>
                <a
                    href="https://docs.google.com/forms/d/e/1FAIpQLSecnSm3UJtQyKB7Th-bLnJchSyYbv1f9RGm1qa3BYNNKaNYYQ/viewform"
                    rel="external"
                    target="_blank"
                    class="inline place-self-center text-blue-500 hover:underline"
                >
                    Then sign up
                </a>
                by 8th April 20.00 UTC
            </p>
        </div>
    }
}
