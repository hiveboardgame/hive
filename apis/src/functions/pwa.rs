#[cfg(feature = "ssr")]
use actix_web::{get, web, HttpResponse, Responder};
use walkdir::WalkDir;

#[cfg(feature = "ssr")]
#[get("/pwa-cache")]
pub async fn cache(site_root: web::Data<String>) -> impl Responder {
    let site_root = site_root.into_inner();
    let site_root = site_root.as_str();
    let assets = WalkDir::new(site_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| path_to_uri(site_root, e.path().to_str().unwrap_or_default()))
        .collect::<Vec<String>>();

    HttpResponse::Ok().json(assets)
}

fn path_to_uri(site_root: &str, path: &str) -> String {
    let relative_path = path.strip_prefix(site_root).unwrap_or(path);
    if relative_path.starts_with("/pkg/") {
        relative_path.to_string()
    } else {
        format!("/assets{}", relative_path)
    }
}
