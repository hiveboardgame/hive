use hive_lib::Piece;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextColor {
    Green,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineContent<'a> {
    Text(&'a str),
    Piece(Piece),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlinePart<'a> {
    pub content: InlineContent<'a>,
    pub bold: bool,
    pub color: Option<TextColor>,
}

/// A small, safe vocabulary for lesson copy. Unknown tokens remain literal text.
/// This never interprets HTML, URLs or arbitrary styling from lesson content.
pub fn inline_parts(mut text: &str) -> Vec<InlinePart<'_>> {
    let mut parts = Vec::new();
    let mut bold = false;
    let mut colors = Vec::new();
    while !text.is_empty() {
        if let Some(rest) = text.strip_prefix("**") {
            bold = !bold;
            text = rest;
            continue;
        }
        let mut tagged = false;
        for (token, color, opening) in [
            ("[green]", TextColor::Green, true),
            ("[/green]", TextColor::Green, false),
            ("[red]", TextColor::Red, true),
            ("[/red]", TextColor::Red, false),
        ] {
            if let Some(rest) = text.strip_prefix(token) {
                if opening {
                    colors.push(color);
                } else if colors.last() == Some(&color) {
                    colors.pop();
                }
                text = rest;
                tagged = true;
                break;
            }
        }
        if tagged {
            continue;
        }
        if let Some(token) = text.strip_prefix("[[") {
            if let Some(end) = token.find("]]") {
                let width = end + 4;
                let content = token[..end]
                    .parse::<Piece>()
                    .map_or(InlineContent::Text(&text[..width]), InlineContent::Piece);
                parts.push(InlinePart {
                    content,
                    bold,
                    color: colors.last().copied(),
                });
                text = &text[width..];
                continue;
            }
        }
        let next = ["**", "[green]", "[/green]", "[red]", "[/red]", "[["]
            .iter()
            .filter_map(|token| text.find(token).filter(|at| *at > 0))
            .min()
            .unwrap_or(text.len());
        parts.push(InlinePart {
            content: InlineContent::Text(&text[..next]),
            bold,
            color: colors.last().copied(),
        });
        text = &text[next..];
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_emphasis_and_icons_can_share_a_sentence() {
        let parts = inline_parts("You play as White. **Move [green]the Queen [[wQ]][/green]**.");
        assert!(parts
            .iter()
            .any(|part| matches!(part.content, InlineContent::Piece(_))
                && part.color == Some(TextColor::Green)
                && part.bold));
        assert_eq!(parts.last().unwrap().color, None);
        assert!(!parts.last().unwrap().bold);
    }
    #[test]
    fn unknown_piece_tokens_remain_text_and_html_is_never_parsed() {
        let parts = inline_parts("[red]<script> [[unknown]][/red] café [[");
        assert!(parts
            .iter()
            .all(|part| matches!(part.content, InlineContent::Text(_))));
        assert_eq!(
            parts
                .iter()
                .filter_map(|part| match part.content {
                    InlineContent::Text(text) => Some(text),
                    _ => None,
                })
                .collect::<String>(),
            "<script> [[unknown]] café [["
        );
    }
}
