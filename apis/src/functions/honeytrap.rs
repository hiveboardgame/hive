use crate::security::csrf::CsrfClient;
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrapHit {
    pub username: String,
    pub ip: String,
    pub source: String,
    pub first_hit: DateTime<Utc>,
    pub last_hit: DateTime<Utc>,
    pub hits: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrapSnapshot {
    pub guest_hits: u64,
    pub guest_lobby_hits: u64,
    pub guest_account_hits: u64,
    pub hits: Vec<TrapHit>,
}

#[server(client = CsrfClient)]
pub async fn read_honeytrap() -> Result<TrapSnapshot, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    Ok(server::snapshot())
}

#[cfg(feature = "ssr")]
pub mod server {
    use super::{TrapHit, TrapSnapshot};
    use actix_identity::Identity;
    use actix_web::{get, web, HttpRequest, HttpResponse};
    use chrono::{DateTime, Utc};
    use db_lib::{get_conn, models::User};
    use std::{
        net::{IpAddr, SocketAddr},
        sync::{LazyLock, Mutex},
    };
    use uuid::Uuid;

    const MAX_ENTRIES: usize = 2048;
    static STORE: LazyLock<Mutex<TrapSnapshot>> = LazyLock::new(Mutex::default);

    pub fn snapshot() -> TrapSnapshot {
        STORE.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn record(
        store: &mut TrapSnapshot,
        username: Option<String>,
        ip: String,
        source: &str,
        now: DateTime<Utc>,
    ) {
        let Some(username) = username else {
            store.guest_hits = store.guest_hits.saturating_add(1);
            if source == "account" {
                store.guest_account_hits = store.guest_account_hits.saturating_add(1);
            } else {
                store.guest_lobby_hits = store.guest_lobby_hits.saturating_add(1);
            }
            return;
        };
        if let Some(hit) = store
            .hits
            .iter_mut()
            .find(|hit| hit.username == username && hit.ip == ip && hit.source == source)
        {
            hit.last_hit = now;
            hit.hits = hit.hits.saturating_add(1);
            return;
        }
        if store.hits.len() == MAX_ENTRIES {
            let oldest = store
                .hits
                .iter()
                .enumerate()
                .min_by_key(|(_, hit)| hit.last_hit)
                .map(|(index, _)| index)
                .unwrap();
            store.hits.swap_remove(oldest);
        }
        store.hits.push(TrapHit {
            username,
            ip,
            source: source.to_owned(),
            first_hit: now,
            last_hit: now,
            hits: 1,
        });
    }

    #[get("/site-index/{source}")]
    pub async fn trap(
        req: HttpRequest,
        source: web::Path<String>,
        identity: Option<Identity>,
        pool: web::Data<db_lib::DbPool>,
    ) -> HttpResponse {
        let source = source.into_inner();
        if !matches!(source.as_str(), "lobby" | "account") {
            return HttpResponse::NotFound().finish();
        }
        // Cross-site embeds must not attribute someone else's request to a signed-in visitor.
        if req.method() != actix_web::http::Method::GET
            || req
                .headers()
                .get("purpose")
                .is_some_and(|v| v == "prefetch")
            || req
                .headers()
                .get("sec-purpose")
                .is_some_and(|v| v.to_str().is_ok_and(|v| v.contains("prefetch")))
            || req
                .headers()
                .get("sec-fetch-site")
                .is_some_and(|v| v == "cross-site")
        {
            return HttpResponse::NoContent()
                .insert_header(("Cache-Control", "no-store"))
                .finish();
        }
        let username = if let Some(identity) = identity {
            let Some(id) = identity.id().ok().and_then(|id| Uuid::parse_str(&id).ok()) else {
                return HttpResponse::NoContent()
                    .insert_header(("Cache-Control", "no-store"))
                    .finish();
            };
            let Ok(mut conn) = get_conn(&pool).await else {
                return HttpResponse::ServiceUnavailable().finish();
            };
            let Ok(user) = User::find_active_by_uuid(&id, &mut conn).await else {
                return HttpResponse::NoContent()
                    .insert_header(("Cache-Control", "no-store"))
                    .finish();
            };
            Some(user.username)
        } else {
            None
        };
        // Use the same proxy-supplied address as password-reset throttling, but retain only a parsed IP.
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .and_then(|s| {
                s.parse::<IpAddr>()
                    .ok()
                    .or_else(|| s.parse::<SocketAddr>().ok().map(|s| s.ip()))
            })
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        record(
            &mut STORE.lock().unwrap_or_else(|e| e.into_inner()),
            username,
            ip,
            &source,
            Utc::now(),
        );
        HttpResponse::NoContent()
            .insert_header(("Cache-Control", "no-store"))
            .finish()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn guests_leave_no_identifying_record() {
            let mut store = TrapSnapshot::default();
            record(&mut store, None, "192.0.2.1".into(), "lobby", Utc::now());
            assert_eq!(store.guest_hits, 1);
            assert!(store.hits.is_empty());
        }
        #[test]
        fn repeated_hits_preserve_first_time_and_distinguish_ip_and_source() {
            let mut store = TrapSnapshot::default();
            let first = Utc::now();
            let last = first + chrono::Duration::seconds(1);
            for now in [first, last] {
                record(
                    &mut store,
                    Some("player".into()),
                    "192.0.2.1".into(),
                    "lobby",
                    now,
                );
            }
            record(
                &mut store,
                Some("player".into()),
                "192.0.2.2".into(),
                "lobby",
                last,
            );
            record(
                &mut store,
                Some("player".into()),
                "192.0.2.1".into(),
                "account",
                last,
            );
            assert_eq!(store.hits.len(), 3);
            assert_eq!(store.hits[0].hits, 2);
            assert_eq!(store.hits[0].first_hit, first);
            assert_eq!(store.hits[0].last_hit, last);
        }
        #[test]
        fn unique_hits_cannot_grow_memory_without_bound() {
            let mut store = TrapSnapshot::default();
            for n in 0..MAX_ENTRIES + 10 {
                record(
                    &mut store,
                    Some(format!("player{n}")),
                    "192.0.2.1".into(),
                    "lobby",
                    Utc::now(),
                );
            }
            assert_eq!(store.hits.len(), MAX_ENTRIES);
        }
    }
}
