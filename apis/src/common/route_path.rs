use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use shared_types::TournamentId;
use url::form_urlencoded::Serializer;

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub(crate) fn tournament_path_matches(pathname: &str, tournament_id: &TournamentId) -> bool {
    let root = format!("/tournament/{}", tournament_id.0);
    pathname
        .strip_prefix(&root)
        .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('/'))
}

pub(crate) fn analysis_url(game_id: Option<&str>, move_query: Option<&str>) -> String {
    let Some(game_id) = game_id else {
        return String::from("/analysis");
    };
    let game_id = utf8_percent_encode(game_id, PATH_SEGMENT);
    let mut url = format!("/analysis/{game_id}");
    if let Some(move_query) = move_query {
        url.push('?');
        url.push_str(
            &Serializer::new(String::new())
                .append_pair("move", move_query)
                .finish(),
        );
    }
    url
}

#[cfg(test)]
mod tests {
    use super::analysis_url;
    use percent_encoding::percent_decode_str;
    use url::Url;

    #[test]
    fn analysis_game_id_stays_in_one_path_segment() {
        let game_id = "game/next?other=1#fragment% é";
        let path = analysis_url(Some(game_id), Some("3"));
        let url = Url::parse(&format!("https://example.test{path}")).expect("valid analysis URL");
        let segments = url
            .path_segments()
            .expect("path segments")
            .collect::<Vec<_>>();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], "analysis");
        assert_eq!(
            percent_decode_str(segments[1]).decode_utf8().unwrap(),
            game_id
        );
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            [("move".into(), "3".into())]
        );
        assert_eq!(url.fragment(), None);
        assert_eq!(
            analysis_url(Some("id-with_underscore"), None),
            "/analysis/id-with_underscore"
        );
    }

    #[test]
    fn analysis_move_query_cannot_add_parameters_or_fragments() {
        let value = "3&foo=1#fragment";
        let path = analysis_url(Some("game_id"), Some(value));
        let url = Url::parse(&format!("https://example.test{path}")).expect("valid analysis URL");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            [("move".into(), value.into())]
        );
        assert_eq!(url.fragment(), None);
    }
}
