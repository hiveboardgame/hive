use linkify::{LinkFinder, LinkKind};

/// Reject link destinations while keeping ordinary plain-text punctuation valid.
pub fn bio_contains_link(bio: &str) -> bool {
    let text: String = bio
        .chars()
        .filter(|c| {
            !matches!(c,
                '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' |
                '\u{2060}'..='\u{206f}' | '\u{feff}'
            )
        })
        .collect();
    let mut finder = LinkFinder::new();
    finder.url_must_have_scheme(false);
    finder.links(&text).any(|link| {
        let address = link.as_str();
        if link.kind() == &LinkKind::Email
            || address.contains("://")
            || address.to_ascii_lowercase().starts_with("www.")
        {
            return true;
        }
        // A bare domain needs a meaningful suffix; e.g. and i.e. are prose.
        let host = address
            .split(['/', ':', '?', '#'])
            .next()
            .unwrap_or(address);
        host.rsplit('.')
            .next()
            .is_some_and(|suffix| suffix.chars().count() >= 2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_web_addresses_and_email_links() {
        for bio in [
            "Visit https://example.com",
            "HTTP://EXAMPLE.COM",
            "www.example.com",
            "example.com/offers",
            "[hello](https://example.com)",
            "<a href=\"https://example.com\">hello</a>",
            "hello@example.com",
            "https://例え.jp",
            "example.co.uk",
            "https://127.0.0.1:3000",
            "https://exa\u{200b}mple.com",
            "exa\u{200b}mple.com",
        ] {
            assert!(bio_contains_link(bio), "allowed a link: {bio}");
        }
    }

    #[test]
    fn allows_normal_biographies_and_abbreviations() {
        for bio in [
            "",
            "Hive enthusiast. Always happy to play!",
            "I enjoy Hive, e.g. casual games.",
            "I play M.L.P. Hive",
            "I am rated 1500. My favourite bug is the beetle.",
            "Hello 🐝 — let's play!",
            "Version 1.2 is fun",
            "I play at 6:30 p.m.",
        ] {
            assert!(!bio_contains_link(bio), "rejected plain text: {bio}");
        }
    }
}
