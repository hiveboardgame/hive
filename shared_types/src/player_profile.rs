use serde::{Deserialize, Serialize};

pub const MAX_BIO_LENGTH: usize = 300;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BioModeration {
    Clear,
    Restrict,
    Replace(String),
    Reinstate,
}
