pub mod presence;
pub mod worker;
pub mod worker_auth;
pub use worker::{eval_claim, eval_fail, eval_progress, eval_result};
pub use worker_auth::EvalWorkerToken;
