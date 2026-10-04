use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tutorial_progress() -> Result<Vec<String>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::TutorialProgress};

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    TutorialProgress::completed_lessons(user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn complete_tutorial_lessons(lessons: Vec<String>) -> Result<(), ServerFnError> {
    use crate::{
        functions::{auth::identity::uuid, db::pool},
        tutorial::find_lesson,
    };
    use db_lib::{get_conn, models::TutorialProgress};

    let user_id = uuid().await?;
    let known: Vec<String> = lessons
        .into_iter()
        .filter(|id| find_lesson(id).is_some())
        .collect();
    if known.is_empty() {
        return Ok(());
    }
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    TutorialProgress::complete(user_id, &known, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}
