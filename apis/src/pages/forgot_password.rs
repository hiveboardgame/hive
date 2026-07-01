use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
    },
    functions::auth::forgot_password::ForgotPassword,
};
use leptos::{form::ActionForm, prelude::*};

#[component]
pub fn ForgotPassword() -> impl IntoView {
    let action = ServerAction::<ForgotPassword>::new();
    let submitted = move || action.value().get().is_some_and(|result| result.is_ok());
    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 sm:p-8">
                <Show
                    when=submitted
                    fallback=move || {
                        view! {
                            <ActionForm action=action attr:class="space-y-4">
                                <div class="flex flex-col gap-1">
                                    <h1 class="ui-page-title">"Forgot your password?"</h1>
                                    <p class="ui-page-subtitle">
                                        "Enter your account email and we'll send you a link to set a new password."
                                    </p>
                                </div>
                                <label class="flex flex-col gap-1.5">
                                    <span class="ui-field-label">"Email"</span>
                                    <input
                                        class="ui-field-input"
                                        name="email"
                                        type="email"
                                        inputmode="email"
                                        autocomplete="email"
                                        placeholder="Email"
                                    />
                                </label>
                                <button
                                    class="w-full ui-button ui-button-primary ui-button-md"
                                    type="submit"
                                >
                                    "Send reset link"
                                </button>
                            </ActionForm>
                        }
                    }
                >
                    <div class="space-y-4">
                        <div class="flex flex-col gap-1">
                            <h1 class="ui-page-title">"Check your inbox"</h1>
                            <p class="ui-page-subtitle">
                                "If an account exists for that email, a password reset link is on its way."
                            </p>
                        </div>
                        <a class="ui-text-link" href="/login">
                            "Back to login"
                        </a>
                    </div>
                </Show>
            </PageCard>
        </PageShell>
    }
}
