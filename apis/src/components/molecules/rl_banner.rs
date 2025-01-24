use leptos::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">King of the Hive</h1>
            <p>
                2025 February Season starts soon! Find out more
                <a
                    href="https://koth.fly.dev/"
                    rel="external"
                    target="_blank"
                    class="place-self-center text-blue-500 hover:underline inline"
                >
                    here!
                </a>
            </p>
            <p>
                <a
                    href="https://koth.fly.dev/register"
                    rel="external"
                    target="_blank"
                    class="place-self-center text-blue-500 hover:underline inline"
                >
                    Sign up
                </a>
                by January 19th 2025
            </p>
        </div>
    }
}
