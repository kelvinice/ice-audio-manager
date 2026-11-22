# Phase 3 Media Core & Art Pipeline Review (`review/media-core`)

> **Reviewer:** Gemini · **Base commit:** `37713e1` (`main`, 2026-11-22) · **Scope:** review only (no code edits)
> **Files reviewed:**
> - [`crates/ice-core/src/media.rs`](../../crates/ice-core/src/media.rs)
> - [`crates/ice-win/src/media.rs`](../../crates/ice-win/src/media.rs)
> - [`spikes/media/Cargo.toml`](../../spikes/media/Cargo.toml)
> - [`spikes/media/src/main.rs`](../../spikes/media/src/main.rs)

---

## 1. Compliance Summary (`code-practices.md`)

| Rule / Area | Status | Notes |
| --- | --- | --- |
| **§1.1–1.4, §1.6 No LL hooks / injection / admin / packers / undocumented APIs** | ✅ Pass | Zero `unsafe` in both `media.rs` modules; uses documented `Windows.Media.Control` (GSMTC) and `Windows.Graphics.Imaging` / `Windows.Storage.Streams` WinRT APIs only. |
| **§1.5 No silent network** | ⚠️ Caution | `thumb.OpenReadAsync()` (`crates/ice-win/src/media.rs:128`) opens an arbitrary app-supplied `IRandomAccessStreamReference`; a URI-backed stream (`CreateFromUri`) from a browser or media player can trigger an OS network fetch or block indefinitely without a size/time cap (`MED-02`, `MED-03`). |
| **§2 Event-driven & heavy work in Rust** | ⚠️ Partial | Event-driven via GSMTC handlers (no polling), with decoding/scaling/accent math in Rust (no extra `image` dependency). However, the art pipeline lacks size and time bounds (`MED-03`, `MED-02`), and `spikes/media:85-87` re-decodes art on unrelated session-list changes (`MED-07`). |
| **§3 Crate layering & `unsafe` hygiene** | ✅ Pass | `ice_core::media` has zero Windows dependencies (`serde` only). Zero `unsafe` blocks and zero non-test `unwrap()` / `expect()` calls in any reviewed file. |
| **§4 WinRT handlers, drop cleanup & threading** | ⚠️ Partial | Event handlers only invoke `sink(signal)` without calling back into GSMTC synchronously, and happy-path `Drop`/`unfollow` remove all 5 tokens. However, **partial handler registration leaks exist on errors** (`MED-01`), **10 WinRT `.join()` calls block unboundedly** (`MED-02`), signal coalescing/backpressure is absent (`MED-07`), and MTA thread constraints are undocumented (`MED-09`). |
| **D-12 Contrast & accent maths** | ❌ Gaps | `readable_accent` / `accent_for_art` uses the wrong WCAG dark/light luminance crossover (`< 0.5` instead of `0.17913`) and hardcodes opaque `ICE_SURFACE` (`#0e1116`), causing contrast below **4.5:1** on translucent/composited notch backgrounds and mid-tone surfaces (`MED-04`). |

---

## 2. Findings by Severity

### High

#### `MED-01` — `crates/ice-win/src/media.rs`: Partial WinRT event-handler leaks on error and missed-event gap on `follow_current()`
- **Where:** `crates/ice-win/src/media.rs:198-214, 222-260`
- **Issue:**
  1. In `MediaWatcher::start`, `mgr.CurrentSessionChanged(...)` registers token 0 (`:201-206`), and if `mgr.SessionsChanged(...)` fails (`:207-210`), `?` returns early without calling `mgr.RemoveCurrentSessionChanged(current)` — leaking a WinRT delegate pointing to `sink` until `mgr` itself dies.
  2. In `MediaWatcher::follow_current`, the closure creates up to 3 session tokens (`MediaPropertiesChanged`, `PlaybackInfoChanged`, `TimelinePropertiesChanged`) and uses `?` inside (`:226-247`). If the 2nd or 3rd subscription fails (very common when a media app closes/switches mid-call), the already-registered tokens are dropped without calling `Remove*`, leaving `self.session = None` so `Drop`/`unfollow` never unregisters them.
  3. Calling `follow_current()` unconditionally calls `self.unfollow()` first (`:223`), tearing down and re-subscribing all 3 session handlers even when `SessionsChanged` fired for an unrelated background tab while the current session stayed the same — opening a missed-event window during tear-down/re-subscribe.
- **Fix:** Store per-handler `Option<i64>` tokens (or small RAII `TokenGuard` helpers) so every registered WinRT handler is always removed on error or `Drop`. Compare session identity (`SourceAppUserModelId` / COM pointer identity) in `follow_current()` before unfollowing and re-subscribing the same active session.

#### `MED-02` — `crates/ice-win/src/media.rs`: Unbounded blocking `.join()` on cross-process GSMTC RPCs and art I/O
- **Where:** `crates/ice-win/src/media.rs:34, 72-76, 93, 102, 128-129, 150, 159, 161, 164`
- **Issue:**
  1. Ten WinRT calls (`RequestAsync`, `TryTogglePlayPauseAsync`..`TrySkipPreviousAsync`, `TryGetMediaPropertiesAsync`, `OpenReadAsync`, `CreateAsync`, `GetPixelDataTransformedAsync`, `BitmapEncoder::CreateAsync`, `FlushAsync`, `LoadAsync`) block synchronously via `.join()` with **no timeout** and never invoke `IAsyncInfo::Cancel()`. Any hung UWP/Electron/browser media app or network/slow thumbnail stream stalls the caller forever (and blocks thread shutdown).
  2. Additionally, tracking changes calls `now_playing()` (`:41`) and then `current_art()` (`:93`), running `session.TryGetMediaPropertiesAsync()?.join()?` twice back-to-back per track change — doubling IPC cost and allowing track metadata (Track A) and thumbnail (Track B) to tear if the track changes in between.
- **Fix:** Add an `IAsyncOperation` / `IAsyncAction` `wait_async(op, timeout)` helper (`SetCompleted` + `Condvar::wait_timeout` + `op.Cancel()` on expiry, e.g. ~250–500 ms for metadata/transport and ~1–2 s total budget for art decode+scale+encode), run all blocking WinRT calls on a dedicated MTA worker thread (`code-practices.md` §4), and combine track + art properties read via a single `MediaProperties` snapshot per change.

#### `MED-03` — `crates/ice-win/src/media.rs`: Missing size bounds in the album art decode/encode pipeline
- **Where:** `crates/ice-win/src/media.rs:91, 126-168`
- **Issue:**
  1. `read_art` does not inspect or bound `stream.Size()` before `BitmapDecoder::CreateAsync(&stream)` (`:129`), nor does it check `decoder.PixelWidth()` / `decoder.PixelHeight()` or total source pixel count before calling `GetPixelDataTransformedAsync` (`:142-151`). A malformed or huge image (or URI-backed stream) can cause unbounded memory allocation / WIC decode time.
  2. `max_px` is unvalidated (`:91, 134`): `max_px == 0` produces `scale = 0.0` clamped up to `(1, 1)`, whereas huge `max_px` permits arbitrarily large buffers.
  3. `encode_png` (`:162-166`) does `let len = u32::try_from(mem.Size()?).unwrap_or(u32::MAX);` and allocates `vec![0u8; len as usize]`, which could request a ~4 GiB allocation on overflow rather than returning an error. It also ignores the `u32` byte count returned by `reader.LoadAsync(len)?.join()?`, risking a partial-read error or uninitialized trailing bytes if `LoadAsync` reads fewer bytes than `len`.
- **Fix:** Enforce explicit ceilings before allocating or decoding (e.g., `1 <= max_px <= 512`, `stream.Size() <= 8 MiB`, source `w.max(h) <= 8192` and `u64::from(w) * u64::from(h) <= 32_000_000`, encoded PNG `mem.Size() <= 512 KiB` returning an error instead of `.unwrap_or(u32::MAX)`), and verify `loaded == len` before `reader.ReadBytes`.

#### `MED-04` — `crates/ice-core/src/media.rs`: Accent contrast math fails D-12 `≥ 4.5:1` on composited notch surfaces and mid-tone surfaces
- **Where:** `crates/ice-core/src/media.rs:41, 142-162`
- **Issue:**
  1. **Wrong WCAG dark/light crossover (`:145`):** `let dark_surface = luminance(surface) < 0.5;` triggers light-text adjustment even on bright mid-tone surfaces above the WCAG black-vs-white crossover (`sqrt(1.05 * 0.05) - 0.05 ≈ 0.17913`). For instance, against `#808080` (`L = 0.2159`), `#999999` (`L = 0.3185`), or `#b0b0b0` (`L = 0.4342`), `readable_accent` pushes lightness to `#ffffff`, yielding contrast ratios of **`3.95:1`**, **`2.85:1`**, and **`2.17:1`** (all below **4.5:1**), whereas black text hits `5.32:1`, `7.37:1`, and `9.68:1`.
  2. **Opaque surface assumption (`:41, 161`):** `accent_for_art` tests strictly against opaque `ICE_SURFACE` (`#0e1116`, `L = 0.00552`). On the translucent notch (`--ice-bg: rgba(14, 17, 22, 0.9)` composited over a white window = `#26292d`, `L = 0.02187`; `--ice-surface: rgba(23, 29, 39, 0.92)` over white = `#2a2f38`, `L = 0.02811`; `Fluent` `rgba(32, 32, 32, 0.88)` over white = `#3b3b3b`, `L = 0.04374`), every accent calibrated only to `4.5–4.8:1` on `#0e1116` drops **below `4.5:1`**:
     - `Rgb(10, 20, 80)` → `#6074eb`: `4.68:1` on `#0e1116`, **`3.61:1`** on `Ice` over white, **`4.18:1`** on opaque `--ice-surface`, **`2.77:1`** on `Fluent` over white.
     - `Rgb(180, 20, 20)` → `#e93a3a`: `4.62:1` on `#0e1116`, **`3.57:1`** on `Ice` over white, **`2.74:1`** on `Fluent` over white.
     - `Rgb(220, 40, 40)` → `#e04242`: `4.53:1` on `#0e1116`, **`3.50:1`** on `Ice` over white, **`2.68:1`** on `Fluent` over white.
     - `Rgb(130, 30, 180)` → `#b355e3`: `4.80:1` on `#0e1116`, **`3.70:1`** on `Ice` over white, **`2.84:1`** on `Fluent` over white.
  3. **Forced lightness floor (`:146`):** `l = l.max(0.55)` washes out mid-tone accents that already pass `4.5:1` on `#0e1116` (e.g. gold `#b8860b` already has `5.81:1` on `#0e1116`, yet is pushed to `#eeb52b` at `10.16:1`), harming art fidelity without improving compliance.
- **Fix:** Choose adjustment direction using `contrast(WHITE, surface) >= contrast(BLACK, surface)` (`luminance(surface) < 0.17913`), calibrate `accent_for_art` against worst-case composited notch surface (`#2a2f38` / theme-aware surface passed in rather than opaque `#0e1116`), and only shift `l` (preferring binary search or fine steps from current `l`) when `contrast(hsl_to_rgb(h, s, l), surface) < 4.5`.

---

### Medium

#### `MED-05` — `crates/ice-win/src/media.rs`: EXIF orientation transposes output buffer vs `(tw, th)`
- **Where:** `crates/ice-win/src/media.rs:130-154`
- **Issue:** `decoder.PixelWidth()` / `PixelHeight()` return **un-oriented** sensor dimensions, and `BitmapTransform` sets `ScaledWidth`/`ScaledHeight` before `ExifOrientationMode::RespectExifOrientation` runs. For 90° or 270° EXIF-rotated non-square art, `DetachPixelData()` returns pixels of shape `(th, tw)` while `ArtInfo` and `encode_png(pixels, tw, th)` treat them as `(tw, th)`, producing sheared/corrupted PNG output or a WIC buffer-size mismatch error.
- **Fix:** Compare `decoder.OrientedPixelWidth()?` / `decoder.OrientedPixelHeight()?` against `(w, h)` to detect 90°/270° transposition and swap `(out_w, out_h)` for `ArtInfo` and `encode_png`, and assert `pixels.len() == (out_w * out_h * 4) as usize` before encoding.

#### `MED-06` — `crates/ice-win/src/media.rs` + `ice-core`: Premultiplied alpha fed to straight-alpha colour maths & PNG encoder
- **Where:** `crates/ice-win/src/media.rs:145, 153, 160` and `crates/ice-core/src/media.rs:88-119`
- **Issue:** `read_art` requests `BitmapAlphaMode::Premultiplied`, then passes those premultiplied BGRA bytes directly into both `accent_for_art` (`dominant_bgra`, which interprets `b, g, r` as un-premultiplied 8-bit channels) and `BitmapEncoder::SetPixelData(..., BitmapAlphaMode::Premultiplied, ...)`, whereas PNG images carry straight (un-premultiplied) alpha and semi-transparent pixels (`128 <= a < 255`) skew darker and alter hue saturation in `dominant_bgra`.
- **Fix:** Request `BitmapAlphaMode::Straight` from `GetPixelDataTransformedAsync` and pass `BitmapAlphaMode::Straight` to `encode_png`.

#### `MED-07` — `crates/ice-win/src/media.rs` + `spikes/media`: Unbounded callback channel and missing coalescing / `SessionsChanged` separation
- **Where:** `crates/ice-win/src/media.rs:198-213` and `spikes/media/src/main.rs:70-93`
- **Issue:**
  1. `MediaSignal::SessionChanged` is emitted by both `CurrentSessionChanged` and `SessionsChanged`. In `spikes/media/src/main.rs:85-87`, every `SessionChanged` tears down/re-subscribes session handlers and runs `current_art` + PNG encode even when an unrelated tab opened or closed (`SessionsChanged`).
  2. `spikes/media` uses an unbounded `mpsc::channel()`, and `MediaWatcher` imposes no signal coalescing across rapid WinRT bursts (e.g., scrubbing timeline/changing tracks rapidly) (`code-practices.md` §4 & §6).
- **Fix:** Split `MediaSignal` (or distinguish `CurrentSessionChanged` vs `SessionsChanged`), coalesce pending signals in the production worker via an atomic dirty bitmask (`AtomicU8`) + wake signal rather than queueing unbounded events, and skip `current_art` when metadata (`title`/`artist`/`album`) or session ID didn't change.

---

### Low

#### `MED-08` — `crates/ice-win/src/media.rs`: `content_id` hashes raw pixels without `(width, height)`
- **Where:** `crates/ice-win/src/media.rs:153`
- **Issue:** `ArtInfo.id` computes `content_id(pixels)` over raw BGRA bytes without encoding `(width, height)`. Two solid-colour or transposed images with identical byte count (`128x64` vs `64x128`) hash to the same `iceart://<id>` cache key.
- **Fix:** Hash the encoded PNG bytes (which embed `width`/`height` in the `IHDR` chunk) or feed `tw.to_le_bytes()` and `th.to_le_bytes()` into `content_id`.

#### `MED-09` — `crates/ice-win/src/media.rs`: Error handling, timeline clamping, and undocumented MTA requirement
- **Where:** `crates/ice-win/src/media.rs:33-54, 96-124`
- **Issue:**
  1. `sessions()` (`:52-54`) uses `.collect::<Result<Vec<_>>>()`, discarding all valid media sessions if one closing session returns an error during `describe()`.
  2. `now_playing()` (`:41-46`) turns any error from `GetCurrentSession()` OR `describe()` into `Ok(None)` (`:44`), masking genuine COM/RPC errors inside `describe()`.
  3. `describe()` (`:111-114`) discards `TimelineProperties::StartTime()` / `LastUpdatedTime()` and does not clamp `position_ms` to `duration_ms` (`ms` uses signed `Duration / 10_000` which turns negative timestamps into `0`, fine, though non-zero `StartTime` offsets will shift `position_ms`/`duration_ms`).
  4. Public blocking functions (`manager`, `now_playing`, `sessions`, `send`, `current_art`) require an MTA (`ComGuard::init_mta()`) and will hang or panic/fail (`RPC_E_WRONG_THREAD`) if called on an STA UI thread (such as Tauri's main window thread), yet only `spikes/media/src/main.rs:35` calls `ComGuard::init_mta()` and this requirement isn't documented in `ice_win::media`'s rustdoc.
- **Fix:** Check `Err` only on `GetCurrentSession()` (propagate `describe()` errors cleanly or log & skip broken sessions in `sessions()`), clamp `position_ms <= duration_ms`, and document the MTA requirement on `ice_win::media` entrypoints.

#### `MED-10` — `crates/ice-core/src/media.rs` & `spikes/media`: Phase 3 TS export / mock trait readiness & trace-level reminder
- **Where:** `crates/ice-core/src/media.rs:9-69`, `crates/ice-win/src/media.rs`, `spikes/media/src/main.rs:94-98`
- **Issue:**
  1. `PlaybackStatus`, `Rgb`, `ArtInfo`, and `NowPlaying` do not yet derive `#[cfg_attr(feature = "ts", derive(TS))]`, and `ice_win::media` exposes free functions + struct `MediaWatcher` without a `MediaBackend` trait for fake testing (`code-practices.md` §4). Acceptable for engine-independent groundwork; add both when wiring Phase 3 into the engine contract.
  2. `spikes/media/src/main.rs:94-98` prints track/artist/album with `println!` (acceptable for the throwaway CLI; when moving to production `app/src-tauri` / `ice-win`, keep all track titles strictly at `tracing::debug!` per `code-practices.md` §4 privacy rule).

---

## 3. Positive Notes
- Zero `unsafe` code and zero `unwrap()`/`expect()` calls in production code across both crates; `ComGuard` (`crates/ice-win/src/com.rs`) cleanly balances `COINIT_MULTITHREADED` + `CoUninitialize`.
- Clean separation between pure `ice_core::media` maths and WinRT `ice_win::media` APIs, reusing built-in Windows WIC (`BitmapDecoder` / `BitmapEncoder`) with zero extra binary/dependency bloat.
- `dominant_bgra()` saturation × mid-lightness weighting cleanly filters out white/grey covers (like Spotify's `#fffffe` case) and falls back deterministically to `ICE_ACCENT`.

---

## 4. Resolution (Claude, `phase-3/media-engine`, 2026-11-23)

| Finding | Resolution |
| --- | --- |
| `MED-01` | Every handler token is owned by a drop guard (`SessionSubs`; the watcher's manager tokens are set one by one on a value that is dropped on error), so a half-finished subscription removes what it registered. `follow_current` keeps the subscriptions when the current session is the same COM object and returns whether it changed. |
| `MED-02` | `wait`/`wait_action`: `SetCompleted` + `recv_timeout` + `Cancel()` on expiry. 500 ms per metadata/transport call, 2 s total for art (shared budget), 2 s for the manager. All calls run on the dedicated `ice-media` MTA thread (`ice_win::media_service`). Track and art come from one `TryGetMediaPropertiesAsync` per change (`media::read`); status/timeline changes only do local reads (`refresh_playback`). |
| `MED-03` | `max_px` clamped to 16–512; stream ≤ 8 MiB, source ≤ 8192 px per side and ≤ 32 M pixels, PNG ≤ 512 KiB (all errors, no `u32::MAX` fallback); `LoadAsync`'s count must equal the length; the pixel buffer length is checked. |
| `MED-04` | Direction chosen by the WCAG crossover (`L < 0.17913` → lighten). An accent that already passes is returned unchanged (no lightness floor). Art accents are calibrated on `ACCENT_SURFACE` `#3d3d3d`, a little lighter than the lightest built-in dark surface (fluent `--ice-surface` over white, `#313135` since `3412900`), so they pass on all of them; tests cover the review's colours on every composited surface and mid-tone surfaces. |
| `MED-05` | `OrientedPixelWidth/Height` detect a 90°/270° EXIF rotation; the output size is swapped accordingly. |
| `MED-06` | Straight alpha for both decode and encode. |
| `MED-07` | `MediaSignal` splits `CurrentSessionChanged` and `SessionsChanged`; the latter only re-reads when the followed session changed. The worker coalesces signals in an `AtomicU8` dirty mask, wakes at most once per burst and reads after a 60 ms settle. Art is still re-read on every properties change (cheap, ~30 ms) because some apps send the new title before the new thumbnail; the PNG is only handed to the engine when its id is new. |
| `MED-08` | `art_id(width, height, pixels)`. |
| `MED-09` | `sessions()` skips sessions that fail to read; `now_playing()` only maps "no current session" to `None`; timeline is relative to `StartTime` and the position is clamped into the track; the module docs state the MTA requirement. |
| `MED-10` | TS derives were added with the media contract (`phase-3/media-api`). Fakes live behind the `Engine` trait (`FakeEngine`) rather than a separate `MediaBackend` trait. Track titles are logged only at `debug`; the `audio-spike media-test` check prints no titles. |

