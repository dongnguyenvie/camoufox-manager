/* Core, loaded first (plain scripts, no bundler):
   - `CM`: registry filled by the other files — CM.components (global components, by tag name)
     and CM.pages (sidebar pages, by key; the nav order lives in app.js).
   - `api(path, opts)`: fetch JSON from the local server, throws Error(message) on failure.
   Classic scripts share ONE global scope: keep file-local helpers inside a `{ ... }` block (see
   pages/profiles.js) so two files never declare the same top-level const. */

const CM = window.CM = { components: {}, pages: {} };

async function api(path, opts = {}) {
  const r = await fetch(path, { headers: { 'content-type': 'application/json' }, ...opts });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) {
    const e = new Error(j.error || r.statusText);
    e.status = r.status;
    throw e;
  }
  return j;
}
