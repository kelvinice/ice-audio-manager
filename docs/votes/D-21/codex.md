# D-21 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---:|---|
| Part A · Rich, items 1–4 together | −1 | The native hit-test alternative looks viable, but the real WebView hit region, animation and long-idle cost are still unmeasured. |
| C0 · Keep existing caps | +2 | The recorded warm WebView fits the visible budget; keeping the target makes a regression visible during the experiment. |
| C1 · Rich-only higher caps | 0 | A separate ceiling could be justified by a measured feature cost; there is no cost breakdown yet for the proposed extras. |
| C2 · Raise every cap | −2 | Relaxing idle CPU and memory for the default profiles would spend the owner's budget without delivering a benefit to those users. |

My component scores for the bundled proposal are **1 −1** for a fourth permanent profile before a usage trial, **2 +2** for configurable linger, **3 0** for live metrics unless they sample only while the Settings page is visible, and **4 +2** for fullscreen fallback and eliminating the always-running poll.

## Strongest argument against my top choice

The warm benchmark covers a settled minute. A heavy media page, GPU-process growth or high DPI may put an opt-in Rich mode over C0 even if its behavior is acceptable to someone who explicitly selected it. A per-profile exception would protect the default modes.

## New option or amendment

Trial “Keep expanded until unpinned” for one session under C0. Measure visible, settled, fullscreen and unpinned states with the Settings metrics page closed. A repeatable budget miss buys a discussion about one specific cap.

## Evidence I checked myself

- On Windows build 26300.9974 I placed a 210×80 physical-pixel layered HWND over another thread's window. Before clipping, the test point at local (170,20) resolved to the overlay. I installed a rounded 95×42 region and created a child button spanning the boundary. In three instrumented trials `WindowFromPoint` at (30,20) found the child and (170,20) found the backing HWND. This routing needed no cursor timer. A native child is only a proxy for WebView2. A region clips paint too, so shadows and spring overshoot need visual checks. [SetWindowRgn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn)
- I traced the existing UI geometry report to `computeHitRect(target)`, immediately after the springs receive their target. Installing that destination region immediately on collapse could clip the pill before the spring settles. The shell's far-cursor branch currently sleeps 100 ms and repeats. [controller.svelte.ts](../../../app/ui/src/lib/notch/controller.svelte.ts#L166), [main.rs](../../../app/src-tauri/src/main.rs#L559)
- I checked the recorded working-set and commit columns separately. The reported warm state is 77.10 MB private working set; the 10× Settings run ends at 167.49 MB private bytes. Neither is a day-long Rich measurement. [Phase 0.2 benchmark](../../spikes/phase0-overlay.md#L55)

## Position

Prototype the host region with the real embedded browser and an unrelated process beneath it. Check idle gaps, shadows, edge clicks and control interaction during expansion and collapse. If a settled visible overlay can sleep within C0, reconsider Rich with that evidence.
