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
