use actix_web::{get, http::header, web::Data, HttpResponse};
use db_lib::{get_conn, DbPool};
use diesel_async::RunQueryDsl;

#[get("/health")]
pub async fn health() -> HttpResponse {
    let mut response = HttpResponse::Ok();
    response.insert_header((header::CACHE_CONTROL, "no-store"));
    if let Ok(addr) = std::env::var("LEPTOS_SITE_ADDR") {
        response.insert_header(("X-Hive-Addr", addr));
    }
    response.body(option_env!("HIVE_RELEASE_SHA").unwrap_or("dev"))
}

#[get("/health/ready")]
pub async fn health_ready(
    pool: Data<DbPool>,
    leptos_options: Data<leptos::prelude::LeptosOptions>,
) -> HttpResponse {
    let mut conn = match get_conn(&pool).await {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().body(format!("db: pool unavailable: {e}"));
        }
    };
    if let Err(e) = diesel::sql_query("SELECT 1").execute(&mut conn).await {
        return HttpResponse::ServiceUnavailable().body(format!("db: ping failed: {e}"));
    }

    let site_root: &str = leptos_options.site_root.as_ref();
    let pkg_dir = std::path::Path::new(site_root).join("pkg");
    let entries = match std::fs::read_dir(&pkg_dir) {
        Ok(e) => e,
        Err(e) => {
            return HttpResponse::ServiceUnavailable()
                .body(format!("assets: {} unreadable: {e}", pkg_dir.display()));
        }
    };
    let (mut has_js, mut has_wasm) = (false, false);
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".js") {
            has_js = true;
        }
        if name.ends_with(".wasm") {
            has_wasm = true;
        }
    }
    if !has_js || !has_wasm {
        return HttpResponse::ServiceUnavailable()
            .body(format!("assets: missing js/wasm in {}", pkg_dir.display()));
    }

    HttpResponse::Ok().body("ready")
}
