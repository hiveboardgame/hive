use crate::responses::GameResponse;

#[derive(Clone, PartialEq)]
pub struct RatingChangeInfo {
    pub white_rating_change: i64,
    pub white_rating: u64,
    pub black_rating_change: i64,
    pub black_rating: u64,
}

impl RatingChangeInfo {
    pub fn from_game_response(gr: &GameResponse) -> Self {
        RatingChangeInfo {
            white_rating_change: gr.white_rating_change.unwrap_or_default() as i64,
            white_rating: gr.white_rating.unwrap_or_default() as u64,
            black_rating_change: gr.black_rating_change.unwrap_or_default() as i64,
            black_rating: gr.black_rating.unwrap_or_default() as u64,
        }
    }
}
