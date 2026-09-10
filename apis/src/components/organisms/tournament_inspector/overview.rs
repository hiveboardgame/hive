use super::{TournamentInspector, TournamentSelection};
use crate::providers::{TournamentCommonStoreFields, TournamentState};
use leptos::prelude::*;
use shared_types::TournamentStatus;

#[component]
pub(crate) fn TournamentOverviewInspector(
    tournament: TournamentState,
    selection: RwSignal<Option<TournamentSelection>>,
    children: ChildrenFn,
) -> impl IntoView {
    let focus_scope_mounted = ArcRwSignal::new(true);
    on_cleanup({
        let focus_scope_mounted = focus_scope_mounted.clone();
        move || focus_scope_mounted.set(false)
    });

    view! {
        <Show
            when=move || {
                selection.get().is_some()
                    && tournament.common.lifecycle().get().status != TournamentStatus::NotStarted
            }
            fallback=move || children()
        >
            <div class="overflow-y-auto overscroll-contain fixed inset-0 z-50 bg-white dark:bg-gray-900 tournament-two:static tournament-two:z-auto tournament-two:max-h-[calc(100dvh-7rem)] tournament-two:bg-transparent dark:tournament-two:bg-transparent">
                <TournamentInspector
                    tournament
                    selection
                    focus_scope_mounted=focus_scope_mounted.clone()
                />
            </div>
        </Show>
    }
}
