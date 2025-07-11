use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum MoveConfirm {
    #[default]
    Double,
    Single,
    Clock,
}

impl fmt::Display for MoveConfirm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            MoveConfirm::Clock => "Clock",
            MoveConfirm::Double => "Double",
            MoveConfirm::Single => "Single",
        };
        write!(f, "{name}")
    }
}

impl FromStr for MoveConfirm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Clock" => Ok(MoveConfirm::Clock),
            "Double" => Ok(MoveConfirm::Double),
            "Single" => Ok(MoveConfirm::Single),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum TileRotation {
    #[default]
    No,
    Yes,
}

impl fmt::Display for TileRotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            TileRotation::Yes => "Yes",
            TileRotation::No => "No",
        };
        write!(f, "{name}")
    }
}

impl FromStr for TileRotation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Yes" => Ok(TileRotation::Yes),
            "No" => Ok(TileRotation::No),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum TileDesign {
    #[default]
    Official,
    Flat,
    ThreeD,
    HighContrast,
    Community,
    Pride,
    Carbon3D,
    Carbon,
}

impl fmt::Display for TileDesign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            TileDesign::Official => "Official",
            TileDesign::Flat => "Flat",
            TileDesign::ThreeD => "ThreeD",
            TileDesign::HighContrast => "HighContrast",
            TileDesign::Community => "Community",
            TileDesign::Pride => "Pride",
            TileDesign::Carbon3D => "Carbon3D",
            TileDesign::Carbon => "Carbon",
        };
        write!(f, "{name}")
    }
}

impl FromStr for TileDesign {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Official" => Ok(TileDesign::Official),
            "Flat" => Ok(TileDesign::Flat),
            "ThreeD" => Ok(TileDesign::ThreeD),
            "HighContrast" => Ok(TileDesign::HighContrast),
            "Community" => Ok(TileDesign::Community),
            "Pride" => Ok(TileDesign::Pride),
            "Carbon3D" => Ok(TileDesign::Carbon3D),
            "Carbon" => Ok(TileDesign::Carbon),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum TileDots {
    #[default]
    Vertical,
    Angled,
    No,
}

impl fmt::Display for TileDots {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            TileDots::Vertical => "Vertical",
            TileDots::Angled => "Angled",
            TileDots::No => "No",
        };
        write!(f, "{name}")
    }
}

impl FromStr for TileDots {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Angled" => Ok(TileDots::Angled),
            "Vertical" => Ok(TileDots::Vertical),
            "No" => Ok(TileDots::No),
            _ => Err(()),
        }
    }
}
