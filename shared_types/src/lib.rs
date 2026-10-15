mod certainty;
mod challenge;
mod chat_capabilities;
mod chat_message;
pub mod clock;
mod conclusion;
mod game_speed;
mod game_start;
mod games_query_options;
mod leaderboard_kind;
mod messages_hub;
mod newtypes;
mod notification_category;
mod notification_channel;
mod opening_explorer;
mod pretty_string;
mod ready_user;
mod reserved_username;
mod schedule;
mod simple_user;
mod takeback_conf;
mod telemetry;
mod time_mode;
mod tournament_configuration;
mod tournament_details;
mod tournament_game_result;
mod tournament_progress;
mod tournament_slot;
mod tournament_snapshot;
mod tournament_start_setup;
pub mod tournament_view;
pub use certainty::{Certainty, RANKABLE_DEVIATION};
pub use challenge::{ChallengeDetails, ChallengeError, ChallengeVisibility};
pub use chat_capabilities::GameChatCapabilities;
pub use chat_message::{
    normalize_chat_message,
    ChatHistoryPage,
    ChatHistoryResponse,
    ChatMessage,
    ChatMessageContainer,
    ConversationKey,
    ConversationUnreadState,
    GameThread,
    MAX_CHAT_MESSAGE_LENGTH,
};
pub use clock::{Clock, ClockPartsError, CorrespondenceClock, RealtimeClock};
pub use conclusion::Conclusion;
pub use game_speed::GameSpeed;
pub use game_start::GameStart;
pub use games_query_options::{
    BatchToken,
    GameProgress,
    GameQueryValidationError,
    GameSort,
    GameSortKey,
    GamesQueryOptions,
    GamesQueryParseError,
    ResultFilter,
    SortValue,
    ALLOWED_BATCH_SIZES,
};
pub use leaderboard_kind::LeaderboardKind;
pub use messages_hub::{
    ChatInboxSnapshot,
    DmConversation,
    GameChannel,
    MessagesCatalogData,
    TournamentChannel,
    MESSAGES_HUB_SECTION_LIMIT,
};
pub use newtypes::{ApisId, ChallengeId, GameId, Password, TournamentId};
pub use notification_category::NotificationCategory;
pub use notification_channel::{CHANNEL_DISCORD, CHANNEL_EMAIL, CHANNEL_PUSH};
pub use opening_explorer::{ExplorerFilters, ExplorerMove, MIN_PLIES};
pub use pretty_string::PrettyString;
pub use ready_user::ReadyUser;
pub use reserved_username::RESERVED_USERNAMES;
pub use schedule::ScheduleOfferStatus;
pub use simple_user::SimpleUser;
pub use takeback_conf::Takeback;
pub use telemetry::{PushMetrics, TelemetryRange, TelemetryRow, TELEMETRY_COLUMN_COUNT};
pub use time_mode::{CorrespondenceMode, TimeMode};
pub use tournament_details::{TournamentDetails, TournamentStatus};
pub use tournament_progress::{SlotAdminAction, SwissProgress, SwissRoundSummary};
pub use tournament_start_setup::TournamentStartSetup;
pub mod tournament {
    pub use crate::{
        clock::{Clock, ClockPartsError, CorrespondenceClock, RealtimeClock},
        tournament_configuration::{
            BotAdmission,
            Config,
            ConfigError,
            DirectEncounterForfeitPolicy,
            Format,
            FormatConfig,
            FormatParseError,
            MatchPointSystem,
            PointSystem,
            ReleasePolicy,
            RepeatedEncounterPolicy,
            TournamentPairingNumberOrder,
            MAX_ELIMINATION_SEATS,
            MAX_ROUND_ROBIN_REPEATS,
            MAX_ROUND_ROBIN_SEATS,
            MAX_SWISS_EXTRA_ROUNDS,
            MAX_SWISS_ROUNDS,
            MAX_SWISS_SEATS,
            MIN_SWISS_EXTRA_ROUNDS,
            MIN_SWISS_ROUNDS,
            MIN_SWISS_START_SEATS,
        },
        tournament_progress::{SlotAdminAction, SwissProgress, SwissRoundSummary},
        tournament_slot::{Resolution, Slot, SlotKey},
        TimeMode,
    };
    pub use tournamint::{
        elimination::EliminationNodeId,
        round_robin::RoundRobinGameId,
        series::SeriesGameId,
        swiss::{SwissGameId, SwissLeg},
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        GameOutcome,
        PlayedGameOutcome,
        Score,
    };

    pub mod arena {
        pub use crate::tournament_configuration::{
            ArenaConfig as Config,
            PairingIntent,
            ARENA_MAX_DURATION_SECONDS as MAX_DURATION_SECONDS,
            ARENA_MIN_DURATION_SECONDS as MIN_DURATION_SECONDS,
        };
    }

    pub mod round_robin {
        pub use crate::tournament_configuration::{
            round_robin_creation_tiebreakers as creation_tiebreakers,
            KoyaOptions,
            RoundRobinConfig as Config,
            RoundRobinCriterion as Criterion,
            RoundRobinDirectEncounterOptions as DirectEncounterOptions,
            RoundRobinPrimaryScore as PrimaryScore,
            RoundRobinProgressiveOptions as ProgressiveOptions,
            RoundRobinSonnebornBergerOptions as SonnebornBergerOptions,
        };
    }

    pub mod swiss {
        pub use crate::tournament_configuration::{
            swiss_creation_tiebreakers,
            DoubleSwissConfig,
            DoubleSwissPrimaryScore as PrimaryScore,
            FlatAcceleration,
            SwissAcceleration as Acceleration,
            SwissBuchholzOptions as BuchholzOptions,
            SwissConfig as Config,
            SwissDirectEncounterOptions as DirectEncounterOptions,
            SwissProgressiveOptions as ProgressiveOptions,
            SwissRoundConfiguration as RoundConfiguration,
            SwissScoreBasis as ScoreBasis,
            SwissSonnebornBergerOptions as SonnebornBergerOptions,
            SwissStandingsCriterion as Criterion,
            SwissSystem as System,
        };
    }

    pub mod standings {
        pub use crate::tournament_snapshot::{Group, Placement, Row, Snapshot, Value};
    }

    pub mod elimination {
        pub use crate::{
            tournament_configuration::{
                ClinchPolicy,
                EliminationConfig as Config,
                EliminationSeriesPhase as SeriesPhase,
                EliminationSeriesPlan as SeriesPlan,
                EliminationStage as Stage,
                EliminationStageOverride as StageOverride,
                EliminationTopology as Topology,
                EntrantSide,
                PlanError,
                SetLimit,
                MAX_ELIMINATION_COLOR_ORDER as MAX_COLOR_ORDER,
                MAX_ELIMINATION_FINITE_SETS as MAX_FINITE_SETS,
                MAX_ELIMINATION_GAMES_PER_SET as MAX_GAMES_PER_SET,
                MAX_ELIMINATION_PHASES as MAX_PHASES,
            },
            tournament_snapshot::{Resolution, Source},
        };
    }
}

pub use tournament_game_result::TournamentGameResult;
