use crate::{components::organisms::header::Header, functions::auth::signup::Signup};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn SignUp(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let signup_action = create_server_action::<Signup>();
    view! {
        <Header/>
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm action=signup_action class="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
                //<div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2">
                        Username

                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                            name="username"
                            type="text"
                            placeholder="Username"
                        />
                    </label>
                //</div>
                //<div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2">
                        Email

                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                            name="email"
                            type="text"
                            placeholder="Email(optional)"
                        />
                    </label>
                //</div>
                //<div class="mb-6">
                    <label class="block text-gray-700 text-sm font-bold mb-2">
                        Password

                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                            name="password"
                            type="password"
                            placeholder="hunter2"
                        />
                    </label>
                //</div>
                //<div class="mb-6">
                    <label
                        class="block text-gray-700 text-sm font-bold mb-2"
                    >
                        Confirm Password

                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                            name="password_confirmation"
                            type="password"
                            placeholder="hunter2"
                        />
                    </label>
                //</div>
                //<div class="flex items-center justify-between">
                    <input
                        type="submit"
                        class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                        value="Sign Up"
                    />

                //</div>
            </ActionForm>

            <a
            class="inline-block align-baseline font-bold text-sm text-blue-500 hover:text-blue-800"
            href="/sign_in"
        >
            Already have an account?
        </a>
        </div>
    }
}
