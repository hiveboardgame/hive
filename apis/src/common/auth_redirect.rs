use url::{form_urlencoded, Position, Url};

pub(crate) fn safe_return_path(path: &str) -> String {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return String::from("/");
    }
    let base = Url::parse("https://hive.invalid/").expect("the local redirect base is valid");
    let Ok(target) = base.join(path) else {
        return String::from("/");
    };
    if target.origin() != base.origin()
        || target.path().starts_with("//")
        || matches!(target.path().trim_end_matches('/'), "/login" | "/register")
    {
        return String::from("/");
    }
    target[Position::BeforePath..].to_owned()
}

pub(crate) fn auth_page_url(page: &str, return_path: &str) -> String {
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", &safe_return_path(return_path))
        .finish();
    format!("{page}?{query}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_destination_stays_local_after_url_normalization() {
        for unsafe_path in [
            "",
            "tournaments/create",
            "https://example.com",
            "//example.com",
            "/\\example.com",
            "/\n/example.com",
            "/a/..//example.com",
            "/login",
            "/register/",
            "/a/../login",
            "/%2e%2e/register",
        ] {
            assert_eq!(safe_return_path(unsafe_path), "/", "{unsafe_path:?}");
        }
        let destination = "/tournaments/create/swiss?example=a%26b#rules";
        assert_eq!(safe_return_path(destination), destination);
        let login = auth_page_url("/login", destination);
        let query = login.split_once('?').unwrap().1;
        let decoded = form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == "return_to")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(safe_return_path(&decoded), destination);
    }
}
