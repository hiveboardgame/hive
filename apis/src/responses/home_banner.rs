use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct HomeBanner {
    pub title: String,
    pub content: String,
}

impl HomeBanner {
    #[cfg(feature = "ssr")]
    pub fn from_model(model: db_lib::models::HomeBanner) -> Option<Self> {
        if model.display {
            Some(Self {
                title: model.title,
                content: model.content,
            })
        } else {
            None
        }
    }
    #[cfg(feature = "ssr")]
    pub fn from_model_ignore_display(model: db_lib::models::HomeBanner) -> Self {
        Self {
            title: model.title,
            content: model.content,
        }
    }
}
