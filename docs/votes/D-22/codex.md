# D-22 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| N0 · Existing network rule | +1 | The offline guarantee is simple to explain and already supports switching and media control. |
| N1 · Per-feature opt-in | +2 | A named provider, precise disclosure and an independent off switch let someone choose whether to share track metadata for lyrics. |
| N2 · General permission | −2 | A single grant could disclose future listening activity to destinations the user never chose. |
| B1 · Control and shortcut | +2 | A user-initiated shortcut to a fixed official URL can reuse the browser's session and the existing Windows media controls. |
| B2 · Embedded website | −1 | Another signed-in WebView adds a cookie store and a remote trust boundary to a shell that already controls the external player. |
| B3 · Unofficial player | −2 | Playback would depend on maintaining private stream extraction in a shipping audio manager whenever the service changes. |
| K0 · No lyrics | 0 | The existing media controls remain useful without lyrics, but some users would choose the privacy trade-off. |
| K1 · LRCLIB | +1 | One named provider offers a bounded first integration if the user enables it and ambiguous recordings can be declined. |
| K2 · Additional providers | −1 | Each provider adds a separate metadata recipient, matching behavior and disclosure to maintain. |
| K3 · Unofficial YouTube lyrics | −2 | Shipping a second client for private endpoints couples a display feature to an unsupported service contract. |

## Strongest argument against my top choice

Even explicit consent to N1 can fade from memory while automatic lookups disclose a listening history one track at a time. B1 also has a smaller downside: opening the site can start a second player when music is already playing elsewhere.

## New option or amendment

Offer a one-track lookup before offering automatic lookup on track changes. Show the destination and fields at that first action. Bind results to the original track, drop a late response after a switch, and let the user clear the local lyrics cache when disabling the feature.

## Evidence I checked myself

- The CSP in `app/src-tauri/tauri.conf.json` restricts connections to the app and local IPC. A narrow native lyrics command can keep that policy and accept the current track identity. The provider URL belongs in the native implementation.
- `crates/ice-core/src/media.rs:78` models playback position and duration as optional milliseconds. I checked Echo's LRCLIB search parameters and duration matching in its reference client. It searches by metadata, compares durations in seconds and widens the tolerance for a fallback. Two releases with similar runtimes can still be different recordings. When timing or identity is uncertain, plain lyrics or a no-match result are preferable to a confidently mistimed line.
- I used the reference only to verify behavior. This vote involved no live track lookup or transmission of listening data.

## Position

Keep playback in the user's chosen player. Allow a narrow opt-in for lyrics after the owner approves the network-rule change. Retain local-only behavior by default and keep third-party destinations out of the notch WebView.
