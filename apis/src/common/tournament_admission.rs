use crate::{
    providers::{AuthContext, AuthIdentity},
    responses::{AccountResponse, TournamentAdmissionViewer},
};
use leptos::prelude::*;
use shared_types::{Clock, GameSpeed};

pub(crate) fn tournament_admission_viewer(
    auth: &AuthContext,
    clock: Option<Clock>,
) -> TournamentAdmissionViewer {
    match auth.identity.get() {
        None => TournamentAdmissionViewer::Pending,
        Some(AuthIdentity::Anonymous) => TournamentAdmissionViewer::Guest,
        Some(AuthIdentity::User(id)) => auth.user.with(|account| {
            account
                .as_ref()
                .filter(|account| account.user.uid == id)
                .map_or(TournamentAdmissionViewer::Pending, |account| {
                    account_admission_viewer(account, clock)
                })
        }),
    }
}

fn account_admission_viewer(
    account: &AccountResponse,
    clock: Option<Clock>,
) -> TournamentAdmissionViewer {
    TournamentAdmissionViewer::User {
        bot: account.user.bot,
        rating: clock.map_or(0.0, |clock| {
            account
                .admission_ratings
                .get(&GameSpeed::from(clock))
                .copied()
                .unwrap_or_default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::responses::{
        AdmissionRestrictions,
        RatingResponse,
        TournamentAccessState,
        TournamentAdmission,
        UserResponse,
        ViewerRelationship,
    };
    use shared_types::{
        tournament::{BotAdmission, RealtimeClock},
        Certainty,
        Takeback,
    };
    use std::{collections::HashMap, num::NonZeroU32};
    use uuid::Uuid;

    #[test]
    fn entry_uses_displayed_rounded_rating_at_band_boundaries() {
        let clock = Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(600).unwrap(),
            increment_seconds: 5,
        });
        let speed = GameSpeed::from(clock);
        let id = Uuid::nil();
        let mut account = AccountResponse {
            username: String::from("entrant"),
            email: String::new(),
            id,
            user: UserResponse {
                username: String::from("entrant"),
                uid: id,
                patreon: false,
                bot: false,
                admin: false,
                deleted: false,
                ratings: HashMap::from([(
                    speed,
                    RatingResponse {
                        speed,
                        rating: 1800,
                        played: 0,
                        win: 0,
                        loss: 0,
                        draw: 0,
                        certainty: Certainty::from_deviation(350.0),
                        user_uid: id,
                    },
                )]),
                takeback: Takeback::Always,
                lang: None,
            },
            admission_ratings: HashMap::from([(speed, 1800.5)]),
        };
        let admission = TournamentAdmission {
            entry_open: true,
            full: false,
            restrictions: AdmissionRestrictions {
                invite_only: false,
                band_lower: Some(1500),
                band_upper: Some(1800),
            },
            relationship: ViewerRelationship {
                joined: false,
                invited: true,
                organizing: false,
            },
            clock: Some(clock),
            bot_admission: BotAdmission::HumansAndBots,
        };
        for (rating, expected) in [
            (1499.4, TournamentAccessState::RatingBelow(1500)),
            (1499.5, TournamentAccessState::PendingInvitation),
            (1499.6, TournamentAccessState::PendingInvitation),
            (1800.4, TournamentAccessState::PendingInvitation),
            (1800.5, TournamentAccessState::RatingAbove(1800)),
        ] {
            account.admission_ratings.insert(speed, rating);
            assert_eq!(
                admission.decision(account_admission_viewer(&account, Some(clock))),
                expected,
                "raw rating: {rating}"
            );
        }
    }
}
