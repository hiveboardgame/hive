use leptos::{html, prelude::*};

#[derive(Clone, Copy)]
pub struct DirectChallengeState {
    pub target: RwSignal<Option<String>>,
    pub dialog_el: NodeRef<html::Dialog>,
}

#[derive(Clone, Copy)]
pub struct DirectChallengeOpener {
    open: Callback<String>,
}

impl DirectChallengeOpener {
    pub fn open(&self, opponent: String) {
        self.open.run(opponent);
    }
}

pub fn provide_direct_challenge() -> DirectChallengeState {
    let state = DirectChallengeState {
        target: RwSignal::new(None),
        dialog_el: NodeRef::new(),
    };

    provide_context(DirectChallengeOpener {
        open: Callback::new(move |opponent| {
            state.target.set(Some(opponent));
            if let Some(dialog) = state.dialog_el.get() {
                let _ = dialog.show_modal();
            }
        }),
    });

    state
}
