# Release scripts

Both scripts read the version from `manager/Cargo.toml`, refuse to run on a dirty git tree (unless told
otherwise), reuse `manager/packaging/bundle-*`, and write to `release/<version>/` (git-ignored).
`SHA256SUMS.txt` is shared: each script replaces only its own lines.

## macOS: `scripts/release-macos.sh`

Prerequisites: macOS, Rust with both targets (`rustup target add aarch64-apple-darwin x86_64-apple-darwin`),
Xcode command line tools (`lipo`, `codesign`, `hdiutil`, `ditto`), network access for the first uv download
(cached in `manager/packaging/.cache/`).

```bash
scripts/release-macos.sh [--arch arm64|x86_64|universal] [--allow-dirty]
```

`universal` (default) builds both targets and merges the app binary and the bundled `uv` with `lipo`.

Outputs: `Camoufox-Manager-<version>-macos-<arch>.dmg` (with an `/Applications` link) and `.zip`.

| Env var | Effect |
| --- | --- |
| `CM_SIGN_IDENTITY` | Developer ID identity; signs with hardened runtime and timestamp. Unset: ad-hoc signing only. |
| `CM_NOTARY_PROFILE` | `notarytool` keychain profile; notarizes and staples the app and dmg. Needs `CM_SIGN_IDENTITY`. |

## Windows: `scripts/release-windows.ps1`

Prerequisites: Windows 10/11 x64, Rust MSVC toolchain, PowerShell 5.1+, git; `signtool` only for signing.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-windows.ps1 [-AllowDirty]
```

Output: `Camoufox-Manager-<version>-windows-x64.zip`.

| Env var | Effect |
| --- | --- |
| `CM_SIGN_CERT` | Path to a `.pfx`; with `CM_SIGN_PASSWORD`, Authenticode-signs `camoufox-manager.exe` (timestamp `http://timestamp.digicert.com`). Unset: unsigned. |
| `CM_SIGN_PASSWORD` | Password of the `.pfx`. |
