use crate::common::{chat_link_href, validated_link_url};
use markdown::{Constructs, Options, ParseOptions};
use url::Url;

pub fn markdown_to_html(markdown: &str) -> Option<String> {
    let options = &Options {
        parse: ParseOptions {
            constructs: Constructs {
                label_start_image: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let html = markdown::to_html_with_options(markdown, options).ok()?;
    let base = Url::parse("https://hivegame.com/").ok()?;
    let mut result = String::with_capacity(html.len());
    // Raw HTML is escaped above. Only compiler-generated anchors can match, and
    // their URL sanitizer percent-encodes quotes/angle brackets and escapes '&'.
    let mut links = html.split("<a href=\"");
    result.push_str(links.next()?);
    for link in links {
        let (href, rest) = link.split_once('"')?;
        let destination = href.replace("&amp;", "&");
        let absolute = Url::parse(&destination);
        let relative = absolute.is_err();
        let url = absolute
            .or_else(|_| base.join(&destination))
            .ok()
            .and_then(|url| validated_link_url(url.as_str()));
        result.push_str("<a");
        if let Some(url) = url.filter(|_| !destination.is_empty()) {
            // Keep local paths relative to the actual page, including in development.
            let href = if relative && url.origin() == base.origin() {
                href.to_string()
            } else {
                chat_link_href(&url)
                    .replace('&', "&amp;")
                    .replace('"', "&quot;")
            };
            result.push_str(" href=\"");
            result.push_str(&href);
            result.push('"');
        }
        result.push_str(" rel=\"noopener noreferrer\"");
        result.push_str(rest);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::markdown_to_html;
    use crate::common::external_link_destination;
    use url::Url;

    #[test]
    fn all_markdown_link_forms_confirm_external_destinations() {
        for markdown in [
            "[inline](https://example.org/path?x=A+B&y=%2B#section)",
            "[reference][site]\n\n[site]: https://example.org/path?x=A+B&y=%2B#section",
            "<https://example.org/path?x=A+B&y=%2B#section>",
            "[relative protocol](//example.org/path?x=A+B&y=%2B#section)",
            "[entities](https://example.org/path?x=A+B&amp;y=%2B#section)",
        ] {
            let html = markdown_to_html(markdown).unwrap();
            let href = html
                .split_once("href=\"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap();
            assert!(href.starts_with("/external-link#"), "{html}");
            assert_eq!(
                external_link_destination(href.split_once('#').unwrap().1),
                Some(Url::parse("https://example.org/path?x=A+B&y=%2B#section").unwrap()),
                "{html}"
            );
            assert!(html.contains("rel=\"noopener noreferrer\""), "{html}");
        }
    }

    #[test]
    fn local_markdown_links_keep_their_relative_destination() {
        for href in ["/tournaments", "../other", "?page=2", "#schedule"] {
            let html = markdown_to_html(&format!("[local]({href})")).unwrap();
            assert!(html.contains(&format!("href=\"{href}\"")), "{html}");
        }
    }

    #[test]
    fn unsupported_destinations_cannot_bypass_confirmation() {
        for href in [
            "javascript:alert(1)",
            "jav&#x61;script:alert(1)",
            "data:text/html,payload",
            "mailto:user@example.org",
            "irc://example.org/channel",
            "https://trusted.example@other.example/path",
            "//trusted.example@other.example/path",
        ] {
            let html = markdown_to_html(&format!("[label]({href})")).unwrap();
            assert!(!html.contains("href="), "{html}");
        }
    }

    #[test]
    fn rewriting_links_preserves_html_escaping() {
        let html = markdown_to_html(
            "<script>alert(1)</script>\n\n<a href=\"https://example.org\">raw</a>\n\n[**label**](https://example.org/?q=%22&copy;=value \"a &quot; title\")",
        )
        .unwrap();
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("<a href=\"https://example.org\""), "{html}");
        assert!(html.contains("&lt;a href="), "{html}");
        assert!(html.contains("<strong>label</strong>"), "{html}");
        assert!(html.contains("title=\"a &quot; title\""), "{html}");
    }
}
