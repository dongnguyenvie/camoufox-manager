# Doctor and notifications

## Doctor

The doctor runs read-only checks on the local setup and reports each one as ok, warning or failed. It
covers, among others, the Camoufox engine, the relay ports, the data folder and Tailscale (only needed for
device sync). Each failed or warning check includes a suggested fix.

- In the app: open the doctor panel.
- From the CLI: `camoufox-manager doctor`, or `doctor --json`.

A report can be saved as Markdown to attach to a bug report. Check it for proxy usernames before sharing.

## Notifications

Events such as a finished scheduled run or a failed check appear in the bell menu in the app. The
bell menu keeps the latest events, each in the selected language.

A macOS banner can also be shown for them. It is off by default.
