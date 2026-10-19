use std::net::IpAddr;
use url::Url;

pub(super) fn validate(value: &str) -> Result<(), String> {
    let url = Url::parse(value)
        .map_err(|error| format!("invalid browser fixture database URL: {error}"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("browser fixture database requires a plain PostgreSQL URL".into());
    }
    let local = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    });
    if !local {
        return Err("browser fixtures require a loopback database host".into());
    }
    let run_id = url
        .path()
        .strip_prefix("/hive_e2e_test_")
        .unwrap_or_default();
    if run_id.len() != 32
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(
            "browser fixtures require a run-owned hive_e2e_test_<32 hex digits> database".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn fixtures_refuse_shared_development_test_seed_and_remote_databases() {
        for value in [
            "postgres://remote.example/hive_e2e_test_0123456789abcdef0123456789abcdef",
            "postgres://127.0.0.1/hive-local",
            "postgres://127.0.0.1/hive-test",
            "postgres://localhost/hive_test",
            "postgres://127.0.0.1/hive-other-test",
            "postgres://127.0.0.1/hive-testware-finally",
            "postgres://127.0.0.1/postgres",
            "postgres://127.0.0.1/template1",
            "postgres://127.0.0.1/hive-template-test",
            "postgres://127.0.0.1/hive-seed-e2e",
            "postgres://127.0.0.1/hive_e2e_test_0123456789abcdef0123456789abcdef?host=remote.example",
            "postgres://127.0.0.1/hive_e2e_test_short",
        ] {
            assert!(validate(value).is_err(), "must refuse {value}");
        }
    }

    #[test]
    fn fixtures_accept_run_owned_databases_only_on_loopback_hosts() {
        for host in ["localhost", "127.0.0.1", "[::1]"] {
            let value =
                format!("postgresql://{host}/hive_e2e_test_0123456789abcdef0123456789abcdef");
            assert!(validate(&value).is_ok());
        }
    }
}
