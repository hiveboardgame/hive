use std::{fs, path::PathBuf};

use actix_web::{get, http::header, web, HttpResponse, Responder};

use sha2::{Digest, Sha256};

const SERVICE_WORKER_TEMPLATE: &str = include_str!("sw-template.js");
const CACHE_NAME_PLACEHOLDER: &str = "__HIVE_CACHE_NAME__";
const ASSETS_PLACEHOLDER: &str = "__HIVE_ASSETS__";

#[derive(Clone)]
pub struct PwaManifest {
    pub cache_version: String,
    assets: Vec<String>,
    worker_script: String,
}

#[derive(Clone)]
pub struct PwaDocumentAssets {
    pub manifest_href: String,
    pub apple_touch_icon_href: String,
    pub pwa_script_src: String,
}

impl PwaManifest {
    pub fn from_site_root(site_root: &str) -> Self {
        let asset_entries = collect_assets(site_root);
        let cache_version = cache_version(&asset_entries);
        let assets = asset_entries
            .into_iter()
            .map(|(asset, _)| asset)
            .collect::<Vec<_>>();
        let worker_script = render_worker_script(&cache_version, &assets);

        Self {
            cache_version,
            assets,
            worker_script,
        }
    }

    pub fn assets(&self) -> &[String] {
        &self.assets
    }

    pub fn worker_script(&self) -> String {
        self.worker_script.clone()
    }

    pub fn document_assets(&self) -> PwaDocumentAssets {
        PwaDocumentAssets::from_cache_version(&self.cache_version)
    }
}

impl PwaDocumentAssets {
    fn from_cache_version(cache_version: &str) -> Self {
        Self {
            manifest_href: versioned_pwa_asset_path("/assets/site.webmanifest", cache_version),
            apple_touch_icon_href: versioned_pwa_asset_path(
                "/assets/android-chrome-192x192.png",
                cache_version,
            ),
            pwa_script_src: versioned_pwa_asset_path("/assets/js/pwa.js", cache_version),
        }
    }
}

fn render_worker_script(cache_version: &str, assets: &[String]) -> String {
    let cache_name = serde_json::to_string(&format!("hivegame-cache-{cache_version}"))
        .expect("serializes cache name");
    let assets = serde_json::to_string(assets).expect("serializes cache manifest");
    let worker_script = SERVICE_WORKER_TEMPLATE
        .replace(CACHE_NAME_PLACEHOLDER, &cache_name)
        .replace(ASSETS_PLACEHOLDER, &assets);

    debug_assert!(!worker_script.contains(CACHE_NAME_PLACEHOLDER));
    debug_assert!(!worker_script.contains(ASSETS_PLACEHOLDER));

    worker_script
}

fn versioned_pwa_asset_path(path: &str, version: &str) -> String {
    if version.is_empty() {
        path.to_string()
    } else {
        format!("{path}?v={version}")
    }
}

#[get("/pwa-cache")]
pub async fn cache(pwa_manifest: web::Data<PwaManifest>) -> impl Responder {
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .json(pwa_manifest.assets())
}

#[get("/assets/js/sw.js")]
pub async fn worker(pwa_manifest: web::Data<PwaManifest>) -> impl Responder {
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header((
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        ))
        .body(pwa_manifest.worker_script())
}

fn collect_assets(site_root: &str) -> Vec<(String, PathBuf)> {
    let mut assets = walkdir::WalkDir::new(site_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let path = entry.into_path();
            (
                path_to_uri(site_root, path.to_str().unwrap_or_default()),
                path,
            )
        })
        .collect::<Vec<_>>();

    assets.sort_by(|left, right| left.0.cmp(&right.0));
    assets
}

fn cache_version(asset_entries: &[(String, PathBuf)]) -> String {
    let mut hasher = Sha256::new();

    for (asset, path) in asset_entries {
        hasher.update(asset.as_bytes());
        hasher.update([0]);

        if let Ok(bytes) = fs::read(path) {
            hasher.update(bytes);
        }

        hasher.update([0xff]);
    }

    format!("{:x}", hasher.finalize())
        .chars()
        .take(16)
        .collect()
}

fn path_to_uri(site_root: &str, path: &str) -> String {
    let relative_path = path.strip_prefix(site_root).unwrap_or(path);
    if relative_path.starts_with("/pkg/") {
        relative_path.to_string()
    } else {
        format!("/assets{relative_path}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        render_worker_script,
        PwaDocumentAssets,
        ASSETS_PLACEHOLDER,
        CACHE_NAME_PLACEHOLDER,
    };

    #[test]
    fn document_assets_are_versioned_from_cache_version() {
        let assets = PwaDocumentAssets::from_cache_version("abc123");

        assert_eq!(assets.manifest_href, "/assets/site.webmanifest?v=abc123");
        assert_eq!(
            assets.apple_touch_icon_href,
            "/assets/android-chrome-192x192.png?v=abc123"
        );
        assert_eq!(assets.pwa_script_src, "/assets/js/pwa.js?v=abc123");
    }

    #[test]
    fn render_worker_script_injects_cache_name_and_assets() {
        let script = render_worker_script(
            "abc123",
            &[
                "/pkg/HiveGame.js".to_string(),
                "/assets/site.webmanifest".to_string(),
            ],
        );

        assert!(script.contains(r#"const CACHE_NAME = "hivegame-cache-abc123";"#));
        assert!(script.contains(
            r#"const ASSETS_TO_CACHE = ["/pkg/HiveGame.js","/assets/site.webmanifest"];"#
        ));
        assert!(!script.contains(CACHE_NAME_PLACEHOLDER));
        assert!(!script.contains(ASSETS_PLACEHOLDER));
    }

    #[test]
    fn render_worker_script_json_escapes_cache_name() {
        let script = render_worker_script("quote\"slash\\", &[]);

        assert!(script.contains(r#"const CACHE_NAME = "hivegame-cache-quote\"slash\\";"#));
        assert!(script.contains("const ASSETS_TO_CACHE = [];"));
    }
}
