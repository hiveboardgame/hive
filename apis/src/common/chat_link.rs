use url::{form_urlencoded, Url};

pub fn validated_link_url(href: &str) -> Option<Url> {
    let (scheme, _) = href.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let url = Url::parse(href).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.host_str()?;
    Some(url)
}

fn is_hivegame_url(url: &Url) -> bool {
    matches!(
        url.origin().ascii_serialization().as_str(),
        "https://hivegame.com" | "https://www.hivegame.com"
    )
}

pub fn chat_link_href(url: &Url) -> String {
    if is_hivegame_url(url) {
        return url.to_string();
    }
    // Fragments keep destination URLs out of Hive's HTTP requests and access logs.
    let fragment = form_urlencoded::Serializer::new(String::new())
        .append_pair("url", url.as_str())
        .finish();
    format!("/external-link#{fragment}")
}

pub fn external_link_destination(hash: &str) -> Option<Url> {
    let fragment = hash.strip_prefix('#').unwrap_or(hash);
    let (_, href) = form_urlencoded::parse(fragment.as_bytes()).find(|(key, _)| key == "url")?;
    // Anyone can construct a confirmation URL without passing through chat parsing.
    validated_link_url(&href).filter(|url| !is_hivegame_url(url))
}

#[cfg(test)]
mod tests {
    use super::{chat_link_href, external_link_destination, validated_link_url};
    use url::{form_urlencoded, Url};

    #[test]
    fn only_the_exact_hivegame_origins_bypass_confirmation() {
        for href in [
            "https://hivegame.com/game/example",
            "HTTPS://HIVEGAME.COM:443/game/example?move=3#history",
            "https://www.hivegame.com/game/example",
        ] {
            let url = validated_link_url(href).unwrap();
            assert_eq!(chat_link_href(&url), url.as_str());
        }
        for href in [
            "http://hivegame.com/game/example",
            "https://hivegame.com:444/game/example",
            "https://www.hivegame.com:444/game/example",
            "https://hivegame.com.evil.example/game/example",
            "https://hivegame.com./game/example",
            "https://evil-hivegame.com/game/example",
            "https://example.org/?next=https://hivegame.com",
        ] {
            let url = validated_link_url(href).unwrap();
            let link = chat_link_href(&url);
            assert!(link.starts_with("/external-link#"), "{href}");
            assert_eq!(
                external_link_destination(link.split_once('#').unwrap().1),
                Some(url),
                "{href}"
            );
        }
    }

    #[test]
    fn confirmation_round_trip_preserves_the_validated_destination() {
        for href in [
            "https://pepeke.app/play/bot?start=1&hop=A+bgA+B-a2=B2+(paM1+S)3-(I-L+Aa2-(QP+g!2=b1+m)),w",
            "https://example.org/a?x=A+B&x=%2B&encoded=%2526&copy;=value#url=https://other.example",
            "https://例え.test/🐝?q=\"quoted\"&encoded=%2f%26#résumé",
        ] {
            let expected = validated_link_url(href).unwrap();
            let link = chat_link_href(&expected);
            let confirmation = Url::parse(&format!("https://hivegame.com{link}")).unwrap();
            assert!(confirmation.query().is_none());
            let actual = external_link_destination(confirmation.fragment().unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn hand_built_confirmation_links_cannot_bypass_url_validation() {
        for href in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "//example.org/path",
            "https://trusted.example@other.example/path",
            "https://user:password@example.org",
            "https:///",
        ] {
            let fragment = form_urlencoded::Serializer::new(String::new())
                .append_pair("url", href)
                .finish();
            assert!(external_link_destination(&fragment).is_none(), "{href}");
        }
        assert!(external_link_destination("").is_none());
        assert!(external_link_destination("url=%25zz").is_none());
    }
}
