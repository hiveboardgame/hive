use crate::responses::{ChallengeResponse, UserResponse};
use shared_types::ChallengeVisibility;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChallengeViewerRole {
    Anonymous,
    Challenger,
    Opponent,
    Other,
}

impl ChallengeViewerRole {
    pub fn is_participant(self) -> bool {
        matches!(self, Self::Challenger | Self::Opponent)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChallengeActionFlags {
    pub accept: bool,
    pub decline: bool,
    pub cancel: bool,
    pub copy_link: bool,
    pub admin_cancel: bool,
}

pub fn challenge_viewer_role(
    challenge: &ChallengeResponse,
    viewer_id: Option<Uuid>,
) -> ChallengeViewerRole {
    let Some(viewer_id) = viewer_id else {
        return ChallengeViewerRole::Anonymous;
    };

    if viewer_id == challenge.challenger.uid {
        ChallengeViewerRole::Challenger
    } else if challenge
        .opponent
        .as_ref()
        .is_some_and(|opponent| viewer_id == opponent.uid)
    {
        ChallengeViewerRole::Opponent
    } else {
        ChallengeViewerRole::Other
    }
}

pub fn challenge_displayed_player(
    challenge: &ChallengeResponse,
    role: ChallengeViewerRole,
) -> (&UserResponse, u64) {
    if role == ChallengeViewerRole::Challenger {
        if let Some(opponent) = challenge.opponent.as_ref() {
            return (opponent, opponent.rating_for_speed(&challenge.speed));
        }
    }

    (&challenge.challenger, challenge.challenger_rating)
}

pub fn challenge_is_viewable(challenge: &ChallengeResponse, role: ChallengeViewerRole) -> bool {
    challenge.visibility != ChallengeVisibility::Direct || role.is_participant()
}

pub fn challenge_action_flags(
    challenge: &ChallengeResponse,
    role: ChallengeViewerRole,
    viewer_is_admin: bool,
    show_private_copy_link: bool,
) -> ChallengeActionFlags {
    if !challenge_is_viewable(challenge, role) {
        return ChallengeActionFlags::default();
    }

    let has_bound_opponent = challenge.opponent.is_some();
    let open_challenge = matches!(
        challenge.visibility,
        ChallengeVisibility::Public | ChallengeVisibility::Private
    ) && !has_bound_opponent;

    ChallengeActionFlags {
        accept: role == ChallengeViewerRole::Opponent
            || (matches!(
                role,
                ChallengeViewerRole::Anonymous | ChallengeViewerRole::Other
            ) && open_challenge),
        decline: role == ChallengeViewerRole::Opponent
            && challenge.visibility == ChallengeVisibility::Direct,
        cancel: role == ChallengeViewerRole::Challenger,
        copy_link: role == ChallengeViewerRole::Challenger
            && challenge.visibility == ChallengeVisibility::Private
            && show_private_copy_link,
        admin_cancel: viewer_is_admin && !role.is_participant(),
    }
}
