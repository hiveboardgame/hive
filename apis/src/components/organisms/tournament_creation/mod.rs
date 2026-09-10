mod editor;
mod model;

pub(crate) use editor::EliminationCreationEditor;
pub(crate) use model::{
    build_elimination_creation_configuration,
    elimination_stages_for_field,
    EliminationCreationDraft,
};
