# Import, export, clone, templates and extensions

## Export and import

A profile can be exported as a zip archive (settings, pinned fingerprint and browser data) and imported
again. When importing, choose the ID mode:

- **New**: always a fresh profile ID (default).
- **Keep if free**: keep the archived ID when it is not used, otherwise use a fresh one.
- **Replace**: overwrite the data of an existing profile. The local ID, group, schedules and proxy
  assignment stay. The old data is backed up, and the write is journalled so a failure does not leave a
  half-written profile. A running profile cannot be replaced.

## Clone and bulk create

Clone copies a profile to a new one. Bulk create makes several profiles from one set of settings.

## Templates

A template is a saved profile configuration. Save a profile as a template, or create profiles from one.

## Extensions

Browser extensions are managed separately and can be attached to profiles from the profile table.

Notes when checking that an extension works (measured on Windows 11 and Camoufox 0.5.9):

- Extensions are installed when the profile starts; a change of the assignment needs a restart.
  No second restart is needed.
- The add-on manifest must have a Gecko id (`browser_specific_settings.gecko.id`).
- A content script with `run_at: document_idle` runs after the page's load event. A script that
  reads the page right after `page.goto()` can see nothing yet; wait or poll for a second.
- Content scripts do not run in a page that was already open before the add-on started, nor in
  `about:blank` (unless the manifest sets `match_about_blank`). Navigate to a page after launch.
