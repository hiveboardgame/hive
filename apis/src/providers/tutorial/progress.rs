use crate::{
    functions::tutorial::{complete_tutorial_lessons, get_tutorial_progress},
    providers::{AuthContext, AuthIdentity},
    tutorial::{find_lesson, lesson_count},
};
use codee::string::FromToStringCodec;
use leptos::{prelude::*, task::spawn_local};
use leptos_use::storage::use_local_storage;
use std::collections::BTreeSet;

const STORAGE_KEY: &str = "tutorial_progress";

#[derive(Clone, Copy)]
pub struct TutorialProgress {
    completed: Memo<BTreeSet<String>>,
    write: WriteSignal<String>,
    identity: Signal<Option<AuthIdentity>>,
}

impl TutorialProgress {
    pub fn local() -> Self {
        let (stored, write, _) = use_local_storage::<String, FromToStringCodec>(STORAGE_KEY);
        let completed = Memo::new(move |_| stored.with(|stored| decode(stored)));
        Self {
            completed,
            write,
            identity: expect_context::<AuthContext>().identity,
        }
    }

    /// Logged-in progress lives in both places; whichever side is missing lessons gets them.
    pub fn synced() -> Self {
        let progress = Self::local();
        Effect::watch(
            move || progress.identity.get(),
            move |identity, _, _| {
                if matches!(identity, Some(AuthIdentity::User(_))) {
                    progress.merge_with_server();
                }
            },
            true,
        );
        progress
    }

    pub fn is_complete(&self, id: &str) -> bool {
        self.completed.with(|completed| completed.contains(id))
    }

    pub fn completed_count(&self) -> usize {
        self.completed.with(BTreeSet::len)
    }

    pub fn total(&self) -> usize {
        lesson_count()
    }

    pub fn first_incomplete(&self) -> Option<&'static str> {
        self.completed.with(|completed| {
            crate::tutorial::lessons()
                .map(|lesson| lesson.id)
                .find(|id| !completed.contains(*id))
        })
    }

    pub fn complete(&self, id: &'static str) {
        if self
            .completed
            .with_untracked(|completed| completed.contains(id))
        {
            return;
        }
        let mut completed = self.completed.get_untracked();
        completed.insert(id.to_string());
        self.write.set(encode(&completed));
        if self.is_logged_in() {
            spawn_local(async move {
                if let Err(error) = complete_tutorial_lessons(vec![id.to_string()]).await {
                    log::warn!("could not save tutorial progress: {error}");
                }
            });
        }
    }

    fn is_logged_in(&self) -> bool {
        matches!(self.identity.get_untracked(), Some(AuthIdentity::User(_)))
    }

    fn merge_with_server(&self) {
        let progress = *self;
        spawn_local(async move {
            let server: BTreeSet<String> = match get_tutorial_progress().await {
                Ok(ids) => ids.into_iter().collect(),
                Err(error) => {
                    log::warn!("could not load tutorial progress: {error}");
                    return;
                }
            };
            let local = progress.completed.get_untracked();
            let missing_on_server: Vec<String> = local.difference(&server).cloned().collect();
            if !missing_on_server.is_empty() {
                if let Err(error) = complete_tutorial_lessons(missing_on_server).await {
                    log::warn!("could not save tutorial progress: {error}");
                }
            }
            let current = progress.completed.get_untracked();
            let merged: BTreeSet<String> = current.union(&server).cloned().collect();
            if merged != current {
                progress.write.set(encode(&merged));
            }
        });
    }
}

fn decode(stored: &str) -> BTreeSet<String> {
    stored
        .split_whitespace()
        .filter(|id| find_lesson(id).is_some())
        .map(str::to_string)
        .collect()
}

fn encode(completed: &BTreeSet<String>) -> String {
    completed.iter().cloned().collect::<Vec<_>>().join(" ")
}
