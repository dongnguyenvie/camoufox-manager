# Cookies

Cookies can be imported into and exported from a profile.

- **Formats**: JSON (an array or object of cookies) and Netscape cookie text. The format is detected
  from the content: JSON starts with `[` or `{`, everything else is read as Netscape text.
- **Export**: download the profile's cookies in either format.
- **Import**: paste or pick a file, then choose how it applies to the profile's existing cookies.
  Cookies that cannot be parsed are reported and skipped.

Cookies contain login sessions, so treat exported files as secrets.
