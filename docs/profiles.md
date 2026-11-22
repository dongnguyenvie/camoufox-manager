# Profiles and fingerprint

A profile is one Camoufox browser identity with its own data folder (`camoufox/profiles/<id>/user-data/`).

## Creating a profile

Choose a name and the operating system of the pinned machine (Windows, macOS or Linux). The app generates
a complete hardware fingerprint once: user agent, platform, CPU, screen, GPU, fonts, audio seed and media
devices. It is stored with the profile.

## Pinned machine

The hardware fingerprint is frozen at creation and never changes, so the profile looks like the same
computer on every launch.

Geography follows the assigned proxy at each launch, unless the profile overrides it:

- timezone and locale
- geolocation (coordinates)
- WebRTC IP

The app always sets timezone together with geolocation, so the host timezone cannot leak.

## Canvas modes

Canvas and WebGL readback noise is controlled per profile:

- **Stable** (default): this profile's own canvas, WebGL and audio fingerprint, the same on every launch.
- **Native**: the real rendering of this machine. Identical on every launch and on every profile, so
  accounts on one machine can be linked by it. Use only if you need it.
- **Random**: Camoufox's own noise, different on every launch.

Known limitation: the first emoji canvas readback in each launch may differ, because Firefox picks a
different font fallback for emoji at startup.

## Starting and stopping

Start opens the browser with the profile's data folder and proxy relay. The table shows the Playwright
WebSocket endpoint for automation (this is the Playwright protocol, not CDP). Stop closes the browser.
Cookies and logins are kept in the profile.

A running profile cannot be replaced by an import or synced; close it first.

## Trash

Deleting a profile moves it to the trash. Its data folder stays on disk, and the profile cannot be started
while it is in the trash. Trashed profiles do not appear in the list, scheduled launches, sheets or sync.

- **Restore** puts it back in the list, stopped.
- **Delete forever** removes the record and its data folder.
- **Empty trash** deletes everything in the trash.

Deleting forever also forgets the profile's sync bookkeeping. If a paired device sends an update for a profile
that is in the trash, the profile is restored and updated (see [sync](sync.md)).

Trashed profiles are kept for `profiles.trash_days` days (default 7, maximum 365). Set it to `0` to delete
immediately, with no trash. Expired profiles are purged at startup and then every hour.

API: `DELETE /api/profiles/{id}` moves a profile to the trash, `GET /api/profiles/trash` lists it,
`POST /api/profiles/{id}/restore` restores it, `DELETE /api/profiles/{id}/purge` deletes it for good, and
`POST /api/profiles/trash/empty` empties the trash.

## Data saving and custom prefs

The **Data** section of a profile has four toggles. All are on by default. Turning one off changes these
Firefox prefs at each launch:

| Toggle off | Firefox prefs |
| --- | --- |
| Passwords | `signon.rememberSignons` and `signon.autofillForms` set to false (no saving or autofill) |
| History | `places.history.enabled` false; history and downloads cleared on shutdown |
| Cache | `browser.cache.disk.enable` false; cache cleared on shutdown |
| Site storage | site storage (localStorage, IndexedDB, service workers, `offlineApps`) cleared by the app before each launch, and on shutdown |

If history, cache or site storage is off, shutdown clearing is turned on. **Cookies and open sessions are
never cleared by these toggles**, so logins are kept.

You can also add custom Firefox prefs (key and value: true/false, a number or text). They are applied last,
so they win over the toggles. Limits:

- at most 200 prefs; names use letters, digits, `.`, `_` and `-` (up to 200 characters)
- values are true/false, a number, or text up to 2000 characters
- these prefix families are reserved and cannot be set (matched in lower case): `network.proxy.`,
  `media.peerconnection.`, `privacy.resistfingerprinting`, `camoufox.`, `geo.` and `intl.`. They are
  managed by the app for the fingerprint, WebRTC, geography, locale and proxy.

## Bookmarks

Each profile can hold a list of bookmarks. They are kept in a toolbar folder named **Camoufox Manager**.
The app manages only that folder. Bookmarks you add by hand in the browser are left alone.

- Before each launch, while the profile is closed, the folder is synced into the profile's `places.sqlite`:
  missing bookmarks are added, removed ones are deleted, and titles and order are updated.
- A profile that has never been opened has no `places.sqlite` yet. The first launch skips the sync, so the
  bookmarks appear from the second start on.
- If the sync fails (for example the database is locked), the browser still starts and the bookmarks are not
  refreshed that time.

New profiles start with 23 default bookmarks, and on upgrade existing profiles with no bookmarks get them too (Facebook, Dropshipping and Crypto folders plus a few loose ones, copied from GoLogin). The list is at `GET /api/bookmarks/defaults`, and the profile form has an **Add defaults** button to add any that are missing.

To import, export your browser's `bookmarks.html` (Netscape format) and send it to
`POST /api/bookmarks/import`. The response is the parsed list. Nothing is saved until you assign it to a
profile.

## Block local network

On by default for each profile (set in the profile settings). It stops the browser from reaching your own
machine and your local network.

- **With a proxy:** the browser is sent through the app's relay for every target, including `localhost` and
  names Firefox would normally send direct. The relay refuses targets on loopback, private, link-local and
  CGNAT addresses, and `localhost` names. The refused request gets `403` and nothing is forwarded.
- **Without a proxy:** the browser still goes through a small local relay that has no upstream and
  connects straight to the target. It applies the same rules, and also resolves names itself: a name that
  resolves to a loopback, private, link-local or CGNAT address is refused too. Location, timezone and
  WebRTC behave as for a profile with no proxy (the relay is not treated as a proxy). The relay starts
  with the profile and stops when it stops or crashes. Firefox's own Local Network Access prefs
  (`network.lna.enabled`, `network.lna.blocking`) are also set; builds without them ignore them.

Limit (with a proxy): the check looks at the target as the browser writes it (an IP address or a name). A
public-looking name that resolves to a private address is not caught, because the proxy resolves names
remotely.


## Search engine

Camoufox ships with every search engine removed and a dummy "None" engine, so typing words in the
address bar goes nowhere. Before each launch the app restores the stock engines in the installed
Camoufox build and makes Google the default (`distribution/policies.json`, `SearchEngines` only).
Firefox reads this policy per install, not per profile, so it applies to every profile; it is reapplied
after a Camoufox update. A policy you edited by hand is left alone.
