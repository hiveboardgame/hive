use crate::i18n::*;
use leptos_i18n::I18nContext;
use shared_types::tournament::Format;

pub fn tournament_format_label(i18n: I18nContext<Locale, I18nKeys>, format: Format) -> String {
    match format {
        Format::RoundRobin => t_string!(i18n, tournaments.format.round_robin),
        Format::Swiss => t_string!(i18n, tournaments.format.swiss),
        Format::DoubleSwiss => t_string!(i18n, tournaments.format.double_swiss),
        Format::SingleElimination => {
            t_string!(i18n, tournaments.format.single_elimination)
        }
        Format::DoubleElimination => {
            t_string!(i18n, tournaments.format.double_elimination)
        }
        Format::Arena => t_string!(i18n, tournaments.format.arena),
    }
    .to_string()
}
