# Cookies

Cookies can be imported into and exported from a profile.

- **Formats**: JSON (an array or object of cookies) and Netscape cookie text. The format is detected
  from the content: JSON starts with `[` or `{`, everything else is read as Netscape text.
- **Export**: download the profile's cookies in either format.
- **Import**: paste or pick a file, then choose how it applies to the profile's existing cookies.
  Cookies that cannot be parsed are reported and skipped.

Cookies contain login sessions, so treat exported files as secrets.

## Cookies through Playwright

`firefox.connect(ws_endpoint)` gives the profile's own context as `browser.contexts[0]`;
`context.add_cookies()` and `context.cookies()` work on it (and on a context made with
`browser.new_context()`, which is separate and not saved in the profile). Two things look like
"no cookies":

- A cookie with a past `expires` is dropped at once. `expires` is in seconds since the epoch, not
  milliseconds.
- A cookie added without `expires` is a session cookie: it is visible while the browser runs but
  is not kept after stop and relaunch. Give it a future `expires` to keep it in the profile.
