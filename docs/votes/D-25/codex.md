# D-25 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| A0 · Static wash | +1 | The existing wash already communicates the cover colours and gives us a useful fallback. |
| A1 · CSS transform drift | +2 | Two softly edged gradients can move independently beneath a stationary scrim using ordinary CSS transforms. |
| A2 · Animate gradient properties | −1 | Changing the gradient itself risks repeated painting, and the current host has no evidence of a visual advantage at this size. |
| A3 · WebGL mesh | −1 | A render loop, graphics context and recovery path add several lifecycle obligations for a decorative detail. |
| A4 · Moving blurred cover | −1 | A moving bright patch behind a short title makes contrast harder to control, and all bundled themes currently disable the blurred cover. |
| B1 · Expanded card | +2 | The expanded card gives two coloured regions room to drift while the user is attending to the controls. |
| B2 · Now-playing pill | −1 | Drift slow enough for the card would barely register during a short popup and could run throughout an entire playlist in persistent mode. |
| B3 · Idle pill | −2 | A continuous animation would introduce recurring frame work in Instant's settled idle state, which has no art backdrop to animate today. |
| C1 · Only while playing | +2 | Pausing playback gives the motion a natural stop condition even on a pinned card. |
| C2 · Stop when hidden, suspended or covered | +2 | The shell already knows the notch monitor and its fullscreen coverage, so it can stop invisible work without cursor polling. |
| C3 · User switch | +2 | Someone who likes spring transitions can turn off sustained background movement independently. |
| C4 · JavaScript frame cap | −1 | A second scheduler would add wakeups and cancellation paths before we know the CSS approach needs one. |
| C5 · Audio-reactive motion | −1 | Meter demand should follow the visualizer, and coupling drift to peaks could make a quiet backdrop distracting on percussion. |
| D1 · One owner-tuned speed | +2 | Compare a few periods at actual notch size and ship one choice that reads well behind the title and buttons. |
| D2 · Three speed presets | −1 | Three public settings add UI and tuning work before we know users can distinguish adjacent speeds on this card. |
| D3 · Theme sets speed | +1 | Theme control gives Minimal a static treatment, though switching themes solely to slow motion would be awkward. |

## Strongest argument against my top choice

An expanded card can stay pinned for hours. Even a transform that avoids repainting can keep the compositor presenting frames for a change a listener hardly sees. On a nearly monochrome cover the movement may be imperceptible. A1 earns its place only if users can see the drift without losing their place while reading.

## New option or amendment

For the owner's speed comparison, keep the foreground still and move one wash centre horizontally by ±12 logical pixels over a full 24-second sine cycle. That gives a peak speed of about 3.1 px/s. Compare 16 and 32 seconds at the same distance on the 384 × 178 card, with the same cover and title. These are prototype choices. Pinning, switching tracks or resuming playback should never make the colours snap across the card. Keep Minimal and reduced motion static. Include the animated-versus-static comparison in the existing Phase 6 pass before release.

## Evidence I checked myself

- In `app/ui/src/notch/activities/NotchBackdrop.svelte`, the image, wash and scrim are siblings inside a layer keyed by art ID. The wash already has two radial gradients. Transforming their parent would also move the contrast scrim. Keying a looping animation there would restart its phase on every track change. Keep the motion on the colour layers and keep the scrim anchored to the text.
- In `app/ui/src/lib/notch/controller.svelte.ts:52`, the UI already combines the Windows reduced-motion flag and Off preset. In `app/src-tauri/src/placement.rs`, fullscreen coverage is checked against the target monitor. Use those existing decisions and update motion eligibility on state changes. Playback alone cannot establish that the card is visible.
- Chromium's animation guide recommends transforms for movement and checking the rendering pipeline. That supports trying A1, but gives no measured GPU or CPU figure for our transparent WebView2 host. [Chrome team's animation guide](https://web.dev/articles/animations-guide)
- W3C explains the five-second threshold for automatic movement alongside other content and why a pause tied only to focus is insufficient. An Appearance switch that also works while the card is pinned gives users lasting control. [W3C guidance on Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html)

## Position

Try transform-driven drift on the expanded playing card, with one tuned speed and an independent off switch. Preserve the shell's existing visibility and motion gates. The implementation needs no new meter subscription, native animation timer or graphics dependency.
