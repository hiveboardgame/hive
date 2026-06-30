pub fn render_password_reset(base_url: &str, username: &str, token: &str) -> (String, String) {
    let link = format!("{base_url}/reset-password?token={token}");
    let body = format!(
        "Hi {username},\n\
         \n\
         We received a request to reset your password. The link expires in 1 hour.\n\
         If you didn't request this, ignore this email — your password has not changed.\n\
         \n\
         {link}\n\
         \n\
         --\n\
         This email was sent by hivegame.com. If you didn't sign up, you can safely\n\
         ignore this message.\n\
         \n\
         Need to talk to a human? Join our Discord and find Ion or leex:\n\
         https://discord.gg/7EwNTJnfab\n"
    );
    ("Reset your password".to_string(), body)
}
