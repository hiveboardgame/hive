# Apis

Apis - The Honey bee!

## Browser request security

Use `#[server(client = CsrfClient)]` on every server function and import
`CsrfClient` and `ActionForm` from `crate::security::csrf`. The client and form
supply tokens automatically; Actix middleware validates CSRF before mutations run.

Debug builds support direct HTTP access through `localhost` or an IP address on
the listening port; the request origin must match the destination exactly.
Release builds trust `https://hivegame.com`. Set `APP_ORIGIN` to override this for
a custom development hostname or deployment. WebSockets require a matching
`Origin`; HTTP mutations allow missing origin headers with a valid CSRF token.

Pending Discord links use a separate encrypted, HttpOnly cookie with a 20-minute
expiry, bound to the user and the session's CSRF token. OAuth handlers must only
read the authentication session: a delayed response must never restore an older
login cookie after logout or an account switch.
