use leptos::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center bg-orange-twilight text-black p-6 mb-3 xs:p-8 xs:mb-4 rounded-sm">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">Join the Rapid League tournament</h1>
            <div class="block">
            Hivegame.com hosts Season 5 of Rapid League!
            Sign up
                                <a
                                    href= "https://forms.gle/UorJRYshLfV8sZFCA"
                                    rel="external"
                                    target="_blank"
                                    class= "text-blue-500 hover:underline"
                                >
                                    here
                                </a>
            before July 15th.
            </div>
        </div>
    }
}
