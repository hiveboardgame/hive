use crate::i18n::Locale;

pub fn parse_locale(code: &str) -> Locale {
    code.parse().unwrap_or_default()
}

#[cfg(feature = "ssr")]
pub fn request_locale(request: &actix_web::HttpRequest) -> Locale {
    if let Some(locale) = request
        .cookie("i18n_pref_locale")
        .and_then(|cookie| cookie.value().parse().ok())
    {
        return locale;
    }
    request
        .headers()
        .get("accept-language")
        .and_then(|header| header.to_str().ok())
        .and_then(|languages| {
            languages.split(',').find_map(|language| {
                let language = language.split(';').next()?.trim();
                language
                    .parse()
                    .ok()
                    .or_else(|| language.split('-').next()?.parse().ok())
            })
        })
        .unwrap_or_default()
}
