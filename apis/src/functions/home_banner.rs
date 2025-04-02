use crate::responses::HomeBanner;
use leptos::prelude::*;

#[server]
pub async fn get() -> Result<Option<HomeBanner>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let banner = models::HomeBanner::get(&mut conn).await?;
    let banner = HomeBanner::from_model(banner);
    Ok(banner)
}

#[server]
pub async fn get_with_display() -> Result<(HomeBanner, bool), ServerFnError> {
    use crate::functions::auth::identity::ensure_admin;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let banner = models::HomeBanner::get(&mut conn).await?;
    let display = banner.display;
    let banner = HomeBanner::from_model_ignore_display(banner);
    Ok((banner, display))
}

#[server]
pub async fn update(title: String, content: String, display: bool) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::ensure_admin;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let mut banner = models::HomeBanner::get(&mut conn).await?;
    banner.title = title;
    banner.content = content;
    banner.display = display;
    banner.update(&mut conn).await?;
    leptos_actix::redirect("/");
    Ok(())
}
