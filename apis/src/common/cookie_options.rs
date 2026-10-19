// Development runs use HTTP; production cookies require HTTPS.
pub(crate) const SECURE_COOKIES: bool = !cfg!(debug_assertions);
