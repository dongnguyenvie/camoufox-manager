# Device sync user guide

Device sync copies profiles between two of your own Macs. It runs over your own Tailscale network. No
third-party server is involved, and the app does not run its own relay.

## Before you start

- Install the [Tailscale](https://tailscale.com/download) app on both Macs and sign in to the same tailnet.
- In the Tailscale admin console, enable **HTTPS certificates** (DNS settings). The Devices page shows
  whether they are on.
- Install Camoufox Manager on both Macs. Both need the same OS, CPU architecture (Apple silicon or Intel)
  and Camoufox build.

If Tailscale is not installed or not running, device sync is disabled and the Devices page shows a hint.
The rest of the app works normally.

## 1. Make each Mac reachable

Open **Settings → Devices** on each Mac. Check the Tailscale status shows **Running**, then choose
**Make this device reachable**. The app runs `tailscale serve` for the sync listener, only after you
confirm. Sync traffic is served over HTTPS on port `8443` (`CM_SYNC_HTTPS_PORT`). Funnel is never used, so
the listener is not exposed to the internet.

If `tailscale serve` cannot be used, the app can fall back to listening on the Tailscale address (100.x)
over plain HTTP. The traffic is still inside the encrypted Tailscale (WireGuard) tunnel, and the Devices
page labels this mode.

## 2. Pair the two Macs

Each Mac shows its pairing link, a QR code and a one-time code. The code is valid for 10 minutes; press
**New code** to make another.

- On the first Mac, show the QR code or copy the link.
- On the second Mac, choose **Add a device**, then paste the link or scan the QR code and enter the code.

Pairing registers a device key on each side. From then on every sync request is signed and checked for
replay, so only paired devices can talk to the listener. Each Mac lists the other under **Paired devices**.

## 3. Sync profiles

Open the device in **Paired devices** and choose **Sync**.

1. Choose a direction: **copy from the other Mac to this one**, or **send from this Mac to the other one**.
2. The app compares both Macs and shows each profile as one of:
   - new on the target
   - changed on one side
   - changed on both sides (a conflict)
   - identical
3. For a conflict, choose one action. Nothing is merged automatically:
   - pull: **Take theirs (replace mine)**, **Save theirs as a copy**, or **Keep mine**
   - push: **Replace theirs with mine**, **Save mine as a copy there**, or **Keep theirs**
4. Start. Each profile shows progress. **Cancel** stops the transfer.

Profiles that are running are listed but cannot be selected. Close them first. Machines with a different OS,
CPU or Camoufox build are listed as not compatible, with the reason.

## What moves

Always:

- profile settings
- the pinned hardware fingerprint
- user data, including cookies and logged-in sessions

Optional:

- **Proxies** (off by default): copies each profile's proxy host, port, type and user name.
- **Proxy password**: copied only if you tick it as well. It travels over the encrypted Tailscale connection.

Transfers are chunked and resumable, with SHA-256 checks on each chunk and on the whole file. The receiving
side applies the copy in a staged, journalled way: a failed or cancelled copy leaves the existing profile
unchanged, and a replaced profile's old data is kept as a backup.

## Profiles in the trash

A profile in the trash is never offered to the other device, whether you pull or push. It still counts as the
same profile when the other device sends it: an incoming update restores it from the trash and applies like
any update for a profile you have (so changes on both sides still show as a conflict, and "Save as a copy" adds
a separate profile and leaves the trashed one alone). If applying the update fails, the profile goes back to
the trash. Deleting a profile forever removes its sync history with every device.

## Revoke a device

Open the device in **Paired devices** and choose **Revoke**. A revoked device can no longer sync. Use
**Remove** to delete it from the list.

## Warning: the same sessions on two Macs

Copying a profile copies its cookies and logged-in sessions. Both Macs then hold the same logins, and
websites may notice, sign you out or flag the accounts. Use each profile on one Mac at a time, and do not
run the same profile on both machines.

The sync dialog asks you to confirm this before the copy starts.
