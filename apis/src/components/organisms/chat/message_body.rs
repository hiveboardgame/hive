use crate::common::{chat_link_href, validated_link_url};
use leptos::{either::Either, prelude::*};
use markdown::{mdast::Node, to_mdast, Constructs, ParseOptions};
use url::Url;

#[derive(Debug, PartialEq)]
enum MessagePart {
    Text(String),
    Link {
        label: String,
        url: Url,
        named: bool,
    },
}

fn message_parts(body: &str) -> Vec<MessagePart> {
    let options = ParseOptions {
        constructs: Constructs {
            attention: false,
            block_quote: false,
            character_reference: false,
            code_indented: false,
            code_fenced: false,
            code_text: false,
            definition: false,
            gfm_autolink_literal: true,
            hard_break_escape: false,
            hard_break_trailing: false,
            heading_atx: false,
            heading_setext: false,
            html_flow: false,
            html_text: false,
            list_item: false,
            thematic_break: false,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    };
    let Ok(root) = to_mdast(body, &options) else {
        return vec![MessagePart::Text(body.to_string())];
    };
    let mut parts = Vec::new();
    let mut offset = 0;
    append_links(&root, body, &mut offset, &mut parts);
    if offset < body.len() {
        parts.push(MessagePart::Text(body[offset..].to_string()));
    }
    parts
}

fn chat_destination_url(href: &str) -> Option<Url> {
    if let Some(url) = validated_link_url(href) {
        return Some(url);
    }
    // Only complete domain-shaped destinations get an implicit HTTPS scheme.
    let authority = href.split(['/', '?', '#']).next()?;
    if !authority.contains('.') || authority.contains('\\') {
        return None;
    }
    let url = validated_link_url(&format!("https://{href}"))?;
    if url.host_str()?.split('.').any(str::is_empty) {
        return None;
    }
    Some(url)
}

fn bare_link_end(body: &str, mut end: usize) -> usize {
    // GFM strips these as Markdown punctuation or character references, but chat
    // URLs need their literal spelling. Leave ordinary sentence punctuation out.
    loop {
        end += body[end..]
            .bytes()
            .take_while(|byte| matches!(byte, b'_' | b'~'))
            .count();
        let Some(name) = body[end..].strip_prefix('&') else {
            return end;
        };
        let name_len = name.bytes().take_while(u8::is_ascii_alphabetic).count();
        if name_len == 0 || name.as_bytes().get(name_len) != Some(&b';') {
            return end;
        }
        end += name_len + 2;
    }
}

fn append_links(node: &Node, body: &str, offset: &mut usize, parts: &mut Vec<MessagePart>) {
    match node {
        Node::Link(link) => {
            let Some(position) = &link.position else {
                return;
            };
            let source = &body[position.start.offset..position.end.offset];
            let named = source.starts_with('[');
            let end = if named || source.starts_with('<') {
                position.end.offset
            } else {
                bare_link_end(body, position.end.offset)
            };
            let source = &body[position.start.offset..end];
            // Only named destinations use Markdown unescaping. Bare URLs must retain
            // their original query strings, including entity-like text and plus signs.
            let href = if named {
                link.url.as_str()
            } else {
                source
                    .strip_prefix('<')
                    .and_then(|s| s.strip_suffix('>'))
                    .unwrap_or(source)
            };
            let Some(url) = chat_destination_url(href) else {
                return;
            };
            if link
                .children
                .iter()
                .any(|child| !matches!(child, Node::Text(_) | Node::Link(_)))
            {
                return;
            }
            // GFM can recognize a URL inside a named label. Flatten it to text
            // rather than rendering a second anchor inside the outer link.
            let label = if named {
                link.children
                    .iter()
                    .map(ToString::to_string)
                    .collect::<String>()
            } else {
                href.to_string()
            };
            if label.trim().is_empty() {
                return;
            }
            // Copy untouched source between links so whitespace and unsupported
            // Markdown keep their literal spelling, even across paragraph boundaries.
            if *offset < position.start.offset {
                parts.push(MessagePart::Text(
                    body[*offset..position.start.offset].to_string(),
                ));
            }
            parts.push(MessagePart::Link { label, url, named });
            *offset = end;
        }
        // Recognize the whole image syntax so it stays literal, including its URL.
        Node::Image(_) => {}
        _ => {
            if let Some(children) = node.children() {
                for child in children {
                    append_links(child, body, offset, parts);
                }
            }
        }
    }
}

#[component]
pub fn ChatMessageBody(body: String) -> impl IntoView {
    let parts = message_parts(&body)
        .into_iter()
        .map(|part| match part {
            MessagePart::Text(text) => Either::Left(text),
            MessagePart::Link { label, url, named } => Either::Right(view! {
                <a
                    href=chat_link_href(&url)
                    target="_blank"
                    rel="noopener noreferrer"
                    class="underline rounded-sm ui-text-link focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
                >
                    <bdi>{label}</bdi>
                    {url
                        .host_str()
                        .filter(|_| named)
                        .map(|hostname| {
                            let hostname = hostname.to_string();
                            view! { <span class="text-xs font-normal">" ↗ · "{hostname}</span> }
                        })}
                </a>
            }),
        })
        .collect_view();

    view! { <span class="whitespace-pre-wrap select-text [overflow-wrap:anywhere]">{parts}</span> }
}

#[cfg(test)]
mod tests {
    use super::{message_parts, ChatMessageBody, MessagePart};
    use leptos::prelude::*;
    use url::Url;

    #[test]
    fn preserves_url_queries_and_surrounding_unicode_and_whitespace() {
        let url = "https://example.org/a_(b)?x=A+B&y=%2B&copy;=value#section";
        let body = format!("Salut 🐝\n\n  ({url}).\nNext");
        assert_eq!(
            message_parts(&body),
            vec![
                MessagePart::Text("Salut 🐝\n\n  (".to_string()),
                MessagePart::Link {
                    label: url.to_string(),
                    url: Url::parse(url).unwrap(),
                    named: false,
                },
                MessagePart::Text(").\nNext".to_string()),
            ]
        );
    }

    #[test]
    fn parses_named_and_bare_links_without_nesting_anchors() {
        assert_eq!(
            message_parts("[https://label.example](https://example.org/a_(b)?q=x+y) https://other.example/path"),
            vec![
                MessagePart::Link {
                    label: "https://label.example".to_string(),
                    url: Url::parse("https://example.org/a_(b)?q=x+y").unwrap(),
                    named: true,
                },
                MessagePart::Text(" ".to_string()),
                MessagePart::Link {
                    label: "https://other.example/path".to_string(),
                    url: Url::parse("https://other.example/path").unwrap(),
                    named: false,
                },
            ],
        );
    }

    #[test]
    fn navigates_to_the_validated_url_without_rewriting_query_values() {
        for (destination, expected) in [
            (
                "HTTPS://EXAMPLE.ORG:443/a/../b?x=A+B&x=%2B&empty=&copy;=value#section",
                "https://example.org/b?x=A+B&x=%2B&empty=&copy;=value#section",
            ),
            (
                "https://例え.test/🐝?q=\"quoted\"&encoded=%2f%26#résumé",
                "https://xn--r8jz45g.test/%F0%9F%90%9D?q=%22quoted%22&encoded=%2f%26#r%C3%A9sum%C3%A9",
            ),
            (
                "http://EXAMPLE.ORG:80/a/%2e%2e/b?redirect=https%3A%2F%2Fother.example%2Fa%3Fx%3D1",
                "http://example.org/b?redirect=https%3A%2F%2Fother.example%2Fa%3Fx%3D1",
            ),
        ] {
            for (body, label, named) in [
                (format!("<{destination}>"), destination, false),
                (format!("[label]({destination})"), "label", true),
            ] {
                assert_eq!(
                    message_parts(&body),
                    vec![MessagePart::Link {
                        label: label.to_string(),
                        url: Url::parse(expected).unwrap(),
                        named,
                    }],
                    "{body}"
                );
            }
        }
    }

    #[test]
    fn preserves_unsupported_markup_and_escaped_links_as_literal_text() {
        let body = "# heading\n**bold** &amp; `code`\n\n![image](https://example.org/image.png)\n\\[label](relative)\n[broken](missing";
        assert_eq!(
            message_parts(body),
            vec![MessagePart::Text(body.to_string())]
        );
    }

    #[test]
    fn unsafe_and_obfuscated_destinations_remain_literal() {
        for destination in [
            "javascript:alert(1)",
            "jav&#x61;script:alert(1)",
            "java&#x09;script:alert(1)",
            "data:text/html,payload",
            "file:///etc/passwd",
            "//example.org/path",
            "https://trusted.example@other.example/path",
        ] {
            let body = format!("[label]({destination})");
            assert_eq!(
                message_parts(&body),
                vec![MessagePart::Text(body.clone())],
                "{body}"
            );
        }
    }

    #[test]
    fn rendering_escapes_untrusted_content_and_isolates_link_navigation() {
        let body = "<script>alert(1)</script> [<img src=x onerror=alert(1)>](https://example.org/?q=\"quoted\") [bad](javascript:alert(1))";
        let html = view! { <ChatMessageBody body=body.to_string() /> }.to_html();
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(html.contains("&lt;img"), "{html}");
        assert!(html.contains("%2522quoted%2522"), "{html}");
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("<img"), "{html}");
        assert!(!html.contains("href=\"javascript:"), "{html}");
        assert!(html.contains("target=\"_blank\""), "{html}");
        assert!(html.contains("rel=\"noopener noreferrer\""), "{html}");
    }

    #[test]
    fn link_label_direction_is_isolated_from_the_hostname() {
        let body = "[label\u{202e}](https://evil.com)";
        let html = view! { <ChatMessageBody body=body.to_string() /> }.to_html();
        assert!(html.contains("<bdi>label\u{202e}"), "{html}");
        let (_, after_label) = html.split_once("</bdi>").unwrap();
        assert!(after_label.contains("evil.com"), "{html}");
    }
}
