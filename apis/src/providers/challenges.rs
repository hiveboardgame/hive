use crate::responses::ChallengeResponse;
use leptos::*;
use shared_types::ChallengeId;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct ChallengeStateSignal {
    pub signal: RwSignal<ChallengeState>,
}

impl Default for ChallengeStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengeStateSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(ChallengeState::new()),
        }
    }

    pub fn remove(&mut self, challenger_id: ChallengeId) {
        self.signal.update(|s| {
            s.challenges.remove(&challenger_id);
        });
    }

    pub fn add(&mut self, challenges: Vec<ChallengeResponse>) {
        self.signal.update(|s| {
            for challenge in challenges {
                s.challenges
                    .insert(challenge.challenge_id.to_owned(), challenge);
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChallengeState {
    pub challenges: HashMap<ChallengeId, ChallengeResponse>,
}

impl ChallengeState {
    pub fn new() -> Self {
        Self {
            challenges: HashMap::new(),
        }
    }
}

impl Default for ChallengeState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_challenges() {
    provide_context(ChallengeStateSignal::new())
}
