use crate::responses::challenge::ChallengeResponse;
use leptos::*;
use std::collections::HashMap;
use uuid::Uuid;

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

    pub fn remove(&mut self, nanoid: String) {
        self.signal.update(|s| {
            s.own.remove(&nanoid);
            s.direct.remove(&nanoid);
            s.public.remove(&nanoid);
        });
    }

    pub fn add(&mut self, challenges: Vec<ChallengeResponse>, user_id: Option<Uuid>) {
        self.signal.update(|s| match user_id {
            Some(id) => {
                for c in challenges {
                    if c.challenger.uid == id {
                        s.own.insert(c.nanoid.to_owned(), c.to_owned());
                        continue;
                    }
                    if let Some(ref opponent) = c.opponent {
                        if opponent.uid == id {
                            s.direct.insert(c.nanoid.to_owned(), c.to_owned());
                            continue;
                        }
                    }
                    s.public.insert(c.nanoid.to_owned(), c.to_owned());
                }
            }
            None => {
                for c in challenges {
                    s.public.insert(c.nanoid.to_owned(), c);
                }
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChallengeState {
    pub public: HashMap<String, ChallengeResponse>,
    pub own: HashMap<String, ChallengeResponse>,
    pub direct: HashMap<String, ChallengeResponse>,
}

impl ChallengeState {
    pub fn new() -> Self {
        Self {
            public: HashMap::new(),
            own: HashMap::new(),
            direct: HashMap::new(),
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
