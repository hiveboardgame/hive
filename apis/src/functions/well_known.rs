// Deep-link verification files for iOS Universal Links and Android App Links.
//
// Apple and Google fetch these once when the app is installed (and re-fetch
// periodically) to verify that this site is allowed to claim its URLs for
// the bound bundle ID. Without them, https://hivegame.com/game/<id> opens
// the browser instead of the installed app.
//
// Strict requirements:
// - Exact paths under /.well-known/, no redirects, served as application/json.
// - Reachable over HTTPS in production. (Dev http: works for testing on
//   simulators/emulators but not for real install-time verification.)
// - assetlinks.json fingerprint must match the signing cert that produced
//   the installed APK — debug cert for sideloads, release cert for Play.
//
// Two placeholder values need to be filled in once the corresponding
// credentials exist (search for "TODO"). The endpoints stay enabled in the
// meantime so the routing scaffolding is testable; Apple/Google just won't
// verify until the placeholders are real.

use actix_web::{get, http::header, HttpResponse, Responder};

// Apple developer team prefix on the bundle ID. Format: "<TEAMID>.<BundleID>"
// where <TEAMID> is the 10-char ASCII string from
// https://developer.apple.com/account → Membership details. Until the team
// is set up, this stays as a placeholder string and Apple won't validate
// the file (the route still serves; Universal Links just won't activate).
// TODO(apple-team-id): replace "APPLETEAM" with the real team prefix.
const APPLE_APP_SITE_ASSOCIATION: &str = r#"{
  "applinks": {
    "details": [
      {
        "appIDs": ["APPLETEAM.com.hivegame.culex"],
        "components": [
          { "/": "/game/*" },
          { "/": "/challenge/*" },
          { "/": "/tournament/*" }
        ]
      }
    ]
  }
}"#;

// SHA-256 fingerprint of the Android signing cert (uppercase hex pairs
// separated by colons). Production cert doesn't exist yet — Phase 4 will
// generate a release keystore. For debug builds, extract via:
//   keytool -list -v -keystore ~/.android/debug.keystore -alias androiddebugkey -storepass android -keypass android
// and copy the "SHA256:" line.
// TODO(android-cert-sha256): replace the placeholder with the real prod
// keystore fingerprint (and optionally include the debug fingerprint as a
// second entry so debug installs verify too).
const ASSETLINKS_JSON: &str = r#"[
  {
    "relation": ["delegate_permission/common.handle_all_urls"],
    "target": {
      "namespace": "android_app",
      "package_name": "com.hivegame.culex",
      "sha256_cert_fingerprints": [
        "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00"
      ]
    }
  }
]"#;

#[get("/.well-known/apple-app-site-association")]
pub async fn apple_app_site_association() -> impl Responder {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body(APPLE_APP_SITE_ASSOCIATION)
}

#[get("/.well-known/assetlinks.json")]
pub async fn assetlinks() -> impl Responder {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body(ASSETLINKS_JSON)
}
