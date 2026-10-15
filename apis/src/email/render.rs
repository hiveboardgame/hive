use crate::i18n::*;

pub fn render_password_reset(
    locale: Locale,
    base_url: &str,
    username: &str,
    token: &str,
) -> (String, String) {
    let link = format!("{base_url}/reset-password?token={token}");
    (
        td_string!(locale, common.auth.reset_your_password).to_string(),
        td_string!(
            locale,
            notifications.email.password_reset_body,
            username = username,
            link = link
        )
        .to_string(),
    )
}
