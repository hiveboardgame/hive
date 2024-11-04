use leptos::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">
                Rapid League season starts Nov 12!
            </h1>
            <div class="flex flex-col">
                <a
                    href="https://docs.google.com/forms/d/e/1FAIpQLSeF5VFfWy2Tfj4KLfn02kRwvlgaP78VH0wWs6kp0yW5adYRxQ/viewform"
                    rel="external"
                    target="_blank"
                    class="place-self-center text-blue-500 hover:underline"
                >
                    Sign up before Nov 12
                </a>
                Flexible schedule of 5-6 matches over 10 weeks with a 10 + 10 time control.
                <div>
                    Schedule your games here or over
                    <a
                        href="https://discord.gg/YwDEmYPHrZ"
                        rel="external"
                        target="_blank"
                        class="text-blue-500 hover:underline"
                    >
                        Discord
                    </a>
                </div>
            </div>
        </div>
    }
}
