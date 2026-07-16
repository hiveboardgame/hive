use crate::i18n::*;
use leptos::prelude::*;
use leptos_router::{components::Outlet, hooks::use_location};

use super::{
    message_path_is,
    sidebar::MessagesSidebar,
    MESSAGES_PRIMARY_HEADER_CLASS,
    MESSAGE_ROOT_PATH,
};

const MESSAGES_SHELL_CLASS: &str = "fixed inset-x-0 bottom-0 top-10 z-0 flex overflow-hidden flex-col border-t border-black/10 bg-light dark:border-white/10 dark:bg-surface-muted sm:flex-row";
const MESSAGES_SIDEBAR_PANE_CLASS: &str = "flex min-h-0 w-full flex-shrink-0 flex-col overflow-hidden border-black/10 bg-light dark:border-white/10 dark:bg-surface-muted sm:w-72 sm:border-r";

#[component]
pub fn MessagesLayout() -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let current_path = Signal::derive(move || location.pathname.get());
    let at_root = Signal::derive(move || message_path_is(&current_path.get(), MESSAGE_ROOT_PATH));
    view! {
        <div class=MESSAGES_SHELL_CLASS>
            <aside class=move || {
                format!(
                    "{MESSAGES_SIDEBAR_PANE_CLASS} {} sm:!flex",
                    if at_root.get() { "" } else { "hidden " },
                )
            }>
                <div class=MESSAGES_PRIMARY_HEADER_CLASS>
                    <h1 class="text-xl ui-page-title">{t!(i18n, messages.page.title)}</h1>
                </div>
                <div class="overflow-y-auto flex-1 p-2 pb-6 min-h-0 sm:pb-2">
                    <MessagesSidebar current_path />
                </div>
            </aside>
            <main class=move || {
                format!(
                    "flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden {} sm:!flex",
                    if at_root.get() { "hidden " } else { "" },
                )
            }>
                <Outlet />
            </main>
        </div>
    }
}
