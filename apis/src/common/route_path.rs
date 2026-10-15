use shared_types::TournamentId;

pub(crate) fn tournament_path_matches(pathname: &str, tournament_id: &TournamentId) -> bool {
    let root = format!("/tournament/{}", tournament_id.0);
    pathname
        .strip_prefix(&root)
        .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('/'))
}
