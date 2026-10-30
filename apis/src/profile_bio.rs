use crate::common::{chat_link_href, validated_link_url};
use linkify::{LinkFinder, LinkKind};

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn bio_to_html(bio: &str) -> String {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    finder.url_must_have_scheme(false);
    let mut html = String::new();
    let mut end = 0;
    for link in finder.links(bio) {
        html.push_str(&escape_html(&bio[end..link.start()]));
        let address = link.as_str();
        let url = validated_link_url(address).or_else(|| {
            if address.contains("://") || address.contains('@') {
                return None;
            }
            let host = address
                .split(['/', ':', '?', '#'])
                .next()
                .unwrap_or(address);
            if host
                .rsplit('.')
                .next()
                .is_some_and(|suffix| suffix.chars().count() < 2)
            {
                return None;
            }
            validated_link_url(&format!("https://{address}"))
        });
        if let Some(url) = url {
            html.push_str(&format!(
                "<a href=\"{}\" rel=\"noopener noreferrer nofollow ugc\">{}</a>",
                escape_html(&chat_link_href(&url)),
                escape_html(address)
            ));
        } else {
            html.push_str(&escape_html(address));
        }
        end = link.end();
    }
    html.push_str(&escape_html(&bio[end..]));
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::external_link_destination;

    #[test]
    fn external_urls_use_existing_confirmation() {
        for text in [
            "https://example.org/path?a=1&b=2",
            "www.example.org",
            "example.org/path",
        ] {
            let html = bio_to_html(text);
            let href = html
                .split_once("href=\"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap()
                .replace("&amp;", "&");
            assert!(href.starts_with("/external-link#"), "{html}");
            assert!(external_link_destination(href.split_once('#').unwrap().1).is_some());
            assert!(html.contains("noopener noreferrer nofollow ugc"));
        }
    }
    #[test]
    fn escapes_user_markup_and_rejects_unsafe_destinations() {
        let html =
            bio_to_html("<script>alert(1)</script>\n<img src=x onerror='alert(1)'> & \"quotes\"");
        assert!(!html.contains("<script") && !html.contains("<img"));
        assert!(
            html.contains("&lt;script&gt;") && html.contains("&amp;") && html.contains("&quot;")
        );
        for text in [
            "javascript:alert(1)",
            "data:text/html,payload",
            "ftp://example.org",
            "https://trusted.example@evil.example/",
        ] {
            assert!(!bio_to_html(text).contains("<a "), "{text}");
        }
    }
    #[test]
    fn preserves_prose_newlines_and_internal_links() {
        assert_eq!(
            bio_to_html("Hello 🐝\nI enjoy Hive, e.g. casual games."),
            "Hello 🐝\nI enjoy Hive, e.g. casual games."
        );
        assert!(bio_to_html("https://hivegame.com/tournaments")
            .contains("href=\"https://hivegame.com/tournaments\""));
    }
}
