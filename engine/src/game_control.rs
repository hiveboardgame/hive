use crate::color::Color;
use crate::game_error::GameError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum GameControl {
    Abort(Color),
    DrawAccept(Color),
    DrawOffer(Color),
    DrawReject(Color),
    Resign(Color),
    TakebackAccept(Color),
    TakebackReject(Color),
    TakebackRequest(Color),
}

impl GameControl {
    pub fn color(&self) -> Color {
        *match self {
            GameControl::Abort(color) => color,
            GameControl::DrawAccept(color) => color,
            GameControl::DrawOffer(color) => color,
            GameControl::DrawReject(color) => color,
            GameControl::Resign(color) => color,
            GameControl::TakebackAccept(color) => color,
            GameControl::TakebackReject(color) => color,
            GameControl::TakebackRequest(color) => color,
        }
    }

    pub fn allowed_on_turn(&self, turn: usize) -> bool {
        match self {
            GameControl::Abort(_) => turn < 2,
            GameControl::DrawAccept(_) => turn > 2,
            GameControl::DrawOffer(_) => turn > 2,
            GameControl::DrawReject(_) => turn > 2,
            GameControl::Resign(_) => turn > 1,
            GameControl::TakebackAccept(_) => turn > 1,
            GameControl::TakebackReject(_) => turn > 1,
            GameControl::TakebackRequest(_) => turn > 2,
        }
    }
}

impl fmt::Display for GameControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let game_control = match self {
            GameControl::Abort(color) => format!("Abort({color})"),
            GameControl::DrawAccept(color) => format!("DrawAccept({color})"),
            GameControl::DrawOffer(color) => format!("DrawOffer({color})"),
            GameControl::DrawReject(color) => format!("DrawReject({color})"),
            GameControl::Resign(color) => format!("Resign({color})"),
            GameControl::TakebackAccept(color) => format!("TakebackAccept({color})"),
            GameControl::TakebackReject(color) => format!("TakebackReject({color})"),
            GameControl::TakebackRequest(color) => format!("TakebackRequest({color})"),
        };
        write!(f, "{game_control}")
    }
}

impl FromStr for GameControl {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Abort(w)" => Ok(GameControl::Abort(Color::White)),
            "Abort(b)" => Ok(GameControl::Abort(Color::Black)),
            "DrawAccept(w)" => Ok(GameControl::DrawAccept(Color::White)),
            "DrawAccept(b)" => Ok(GameControl::DrawAccept(Color::Black)),
            "DrawOffer(w)" => Ok(GameControl::DrawOffer(Color::White)),
            "DrawOffer(b)" => Ok(GameControl::DrawOffer(Color::Black)),
            "DrawReject(w)" => Ok(GameControl::DrawReject(Color::White)),
            "DrawReject(b)" => Ok(GameControl::DrawReject(Color::Black)),
            "Resign(w)" => Ok(GameControl::Resign(Color::White)),
            "Resign(b)" => Ok(GameControl::Resign(Color::Black)),
            "TakebackAccept(w)" => Ok(GameControl::TakebackAccept(Color::White)),
            "TakebackAccept(b)" => Ok(GameControl::TakebackAccept(Color::Black)),
            "TakebackRequest(w)" => Ok(GameControl::TakebackRequest(Color::White)),
            "TakebackRequest(b)" => Ok(GameControl::TakebackRequest(Color::Black)),
            "TakebackReject(w)" => Ok(GameControl::TakebackReject(Color::White)),
            "TakebackReject(b)" => Ok(GameControl::TakebackReject(Color::Black)),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "GameControl string".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_game_controls() {
        for gc in [
            GameControl::Abort(Color::White),
            GameControl::DrawAccept(Color::White),
            GameControl::DrawOffer(Color::White),
            GameControl::DrawReject(Color::White),
            GameControl::Resign(Color::White),
            GameControl::TakebackAccept(Color::White),
            GameControl::TakebackReject(Color::White),
            GameControl::TakebackRequest(Color::White),
            GameControl::Abort(Color::Black),
            GameControl::DrawAccept(Color::Black),
            GameControl::DrawOffer(Color::Black),
            GameControl::DrawReject(Color::Black),
            GameControl::Resign(Color::Black),
            GameControl::TakebackAccept(Color::Black),
            GameControl::TakebackReject(Color::Black),
            GameControl::TakebackRequest(Color::Black),
        ]
        .iter()
        {
            assert_eq!(Ok(gc.clone()), GameControl::from_str(&format!("{gc}")));
        }
    }
}
