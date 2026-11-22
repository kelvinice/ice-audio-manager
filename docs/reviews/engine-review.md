# Phase 1 Engine & Core Code Review (`review/engine`)

> **Reviewer:** Gemini · **Base commit:** `fe095ea` (`main`, 2026-11-22) · **Scope:** review only (no code edits)
> **Files reviewed:**
> - `crates/ice-win/src/{audio_engine,file_watch,hotkeys}.rs` and direct Win32/COM/WinRT helpers (`com.rs`, `endpoints.rs`, `notify.rs`, `policy.rs`, `spatial.rs`, `volume.rs`)
> - `crates/ice-core/src/{resolve,config,engine,hotkey}.rs`

---

## 1. Compliance Summary (`code-practices.md`)

| Rule / Area | Status | Notes |
| --- | --- | --- |
| **§1.1 No LL input hooks** | ✅ Pass | Zero `WH_KEYBOARD_LL` / `WH_MOUSE_LL` or raw-input keylogging. Global chords & F31 knob modifiers use `RegisterHotKey` (`hotkeys.rs:104`); `Chord::from_str` rejects unmodified `0xAD..=0xB3` media/volume keys (`hotkey.rs:134-136`). Volume comes from `IAudioEndpointVolumeCallback`. |
| **§1.2–1.5 No injection / admin / packer / silent network** | ✅ Pass | Zero process injection, zero elevation, zero network calls in `ice-core` or `ice-win`. Config persistence touches only the caller-provided config path (`config.rs:255-262`). |
| **§1.6 Undocumented `IPolicyConfig` guarded** | ✅ Pass | Isolated in `crates/ice-win/src/policy.rs`; failures surface cleanly through `Result` into `Notice::SwitchFailed` (`audio_engine.rs:673-676`). |
| **§3 Crate layering & `unsafe` hygiene** | ✅ Pass | Zero Windows deps and zero `unsafe` in `ice-core`; zero `unsafe` in `audio_engine.rs`. Every `unsafe` block in `file_watch.rs`, `hotkeys.rs`, `com.rs`, `notify.rs`, `volume.rs`, `policy.rs`, and `endpoints.rs` carries a `// SAFETY:` comment. |
| **§4 COM threading & handle lifetime** | ⚠️ Partial | Core Audio callbacks (`notify.rs:31-66`, `volume.rs:21-29`) are strictly non-blocking (`tx.send()` only), COM objects stay on the MTA `ice-audio` thread, and `Worker._com` drops last (`audio_engine.rs:188`). However, **stall/loop hazards exist in `hotkeys.rs` and `spatial::set_default()`**, detached `ice-forward` threads are leaked (`ENG-05`), panicking `DirWatch` callbacks leak a Win32 handle (`ENG-09`), and stale `VolumeWatch` handles are not refreshed on same-ID reconnects (`ENG-07`). |

---

## 2. Findings by Severity

### High

#### `ENG-01` — `hotkeys.rs`: `GetMessageW` `-1` infinite loop + ignored `PostThreadMessageW` errors can stall `ice-audio` or hang `Drop`
- **Where:** `crates/ice-win/src/hotkeys.rs:52-56, 62-66, 70-75, 91, 104`
- **Issue:**
  1. `while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool()` treats `-1` (`BOOL(-1)`, returned on error) as `true` (`!= 0`), spinning at 100% CPU forever and never reaching the cleanup loop at lines 121–124.
  2. `push()` ignores `PostThreadMessageW` failures (`hotkeys.rs:65`), while `register()` immediately waits on `rx.recv()` (`hotkeys.rs:55`) with no timeout. If the hotkey thread exits or its message queue rejects the message, `ice-audio` hangs permanently on `reload_config()` or startup.
  3. `HotkeyService::drop` ignores `PostThreadMessageW(..., WM_QUIT, ...)` errors (`hotkeys.rs:72`) and unconditionally calls `j.join()`.
  4. `RegisterHotKey(None, ...)` registers thread-level hotkeys instead of using a hidden message-only (`HWND_MESSAGE`) window as specified in `plan.md` line 114; thread messages sent via `PostThreadMessageW` can be dropped if any modal loop later runs on that thread.
- **Fix:** Check `ret.0 > 0` on `GetMessageW` (matching standard Win32 `while (ret = GetMessageW(...)) != 0 { if ret == -1 { break; } ... }`); host hotkeys on an `HWND_MESSAGE` window and post commands via `PostMessageW`; check `PostMessageW` return values and return an error (or bounded `recv_timeout`) from `register()` if dispatch fails.

#### `ENG-02` — `audio_engine.rs` + `spatial.rs`: Blocking WinRT `.join()` and redundant per-endpoint `spatial::status()` probes stall `ice-audio`
- **Where:** `crates/ice-win/src/audio_engine.rs:330-351, 367, 393, 402, 671, 703, 735-741` and `crates/ice-win/src/spatial.rs:115-127, 144`
- **Issue:**
  1. `spatial::set_default()` blocks `ice-audio` synchronously on `cfg.SetDefaultSpatialAudioFormatAsync(&subtype)?.join()` (`spatial.rs:144`) with no timeout; a hung audio/Dolby provider freezes every hotkey, volume event, and `AudioEngine::drop()`.
  2. `Worker::summary()` invokes `spatial::status(&e.id)` (`audio_engine.rs:333`), which queries WinRT `GetForDeviceId` + `IsSpatialAudioSupported` + `ActiveSpatialAudioFormat` + `DefaultSpatialAudioFormat` **plus all 7 `IsSpatialAudioFormatSupported` calls** even though `summary()` discards `available`, `supported`, and `default`!
  3. Worse, `refresh(true)` calls `summary()` multiple times for the same endpoints: once for `visible` (`:367`), a second time for every `arrived` device (`:393`), and once on every disconnected `left` device (`:402`) only to discard `spatial` with `spatial: None` (`:401`)! A single `switch()` also runs `summary(target)` (`:671`), `refresh(true)` (`:701`), `summary(e)` (`:703`), plus up to two `spatial::status()` calls inside `apply_spatial()` (`:736, 741`).
- **Fix:** Split a fast `spatial::active_format(id)` helper that skips the 7 `IsSpatialAudioFormatSupported` calls for summaries; compute `summary()` once per endpoint during `refresh()` and never query WinRT spatial status on disconnected `left` endpoints; consider timeout/offloading for `SetDefaultSpatialAudioFormatAsync` per Phase 6 watchdog goals.

#### `ENG-03` — `audio_engine.rs`: Raw `Vec` indices in `PendingChoice` go stale across `reload_config()`
- **Where:** `crates/ice-win/src/audio_engine.rs:143-146, 461, 593-599, 630, 707-719`
- **Issue:** `Loc::Binding(usize)` and `Loc::Pair(usize, usize)` capture vector indices when emitting `Notice::ChooseDevice` and reuse those indices later when `Command::ResolveChoice` arrives. If a `config.toml` hot-reload replaces `self.config` (`:461`) while a device picker is open (reordering, adding, or deleting bindings/toggle pairs), `persist_relink()` mutates the **wrong** binding or toggle pair. Also, `self.pending` entries are never expired or pruned if the user cancels without sending `ResolveChoice { device_id: None }`.
- **Fix:** Key `Loc` by `binding_id: String` / `pair_id: String` (matching how commands already identify them) and clear `self.pending` whenever `reload_config()` swaps `self.config` (plus cap or prune timed-out requests).

#### `ENG-04` — `config.rs` + `audio_engine.rs`: `ConfigStore::save()` is fragile on Windows locks, drops errors, and can clobber concurrent external edits
- **Where:** `crates/ice-core/src/config.rs:231-236, 255-262` and `crates/ice-win/src/audio_engine.rs:726`
- **Issue:**
  1. `ConfigStore::save()` writes `.toml.tmp` without flushing and calls `std::fs::rename(&tmp, &self.path)` (`config.rs:259-261`). On Windows, `rename` over an existing file fails with `ERROR_ACCESS_DENIED` / `ERROR_SHARING_VIOLATION` when an editor or Defender holds a brief handle, leaving a stray `.toml.tmp` behind and failing the save — and `persist_relink()` discards the error via `let _ = self.store.save(&self.config);` (`audio_engine.rs:726`).
  2. Writing directly in `persist_relink()` also ignores `self.reload_at`: if the user just edited `config.toml` (waiting out the 300 ms `RELOAD_DEBOUNCE`) when a hotkey triggers a re-link, `save()` overwrites the user's unsaved external changes (violating `code-practices.md` §7 conflict detection).
  3. `ConfigStore::replace_broken()` discards `std::fs::rename(&self.path, &backup)` (`config.rs:234`) while still reporting `Loaded::Replaced { backup }` as if the backup succeeded, and uses 1-second Unix timestamps that can collide on rapid restarts.
- **Fix:** Surface `ConfigStore::save()` errors via a user notice / log; sync `.tmp` before replacing with `ReplaceFileW` / short lock retry + clean up `.tmp` on failure; record last-loaded file metadata/hash before overwriting in `persist_relink()`; check rename success in `replace_broken()`.

---

### Medium

#### `ENG-05` — `audio_engine.rs`: Detached `ice-forward` threads leak on config reload and can outlive `AudioEngine::drop()`
- **Where:** `crates/ice-win/src/audio_engine.rs:192-203, 209, 462-464, 497`
- **Issue:** `forward()` spawns an unjoined `ice-forward` thread (`let _ = Builder::spawn(...)`) solely to wrap `AudioEvent`/`i32` into `Msg`.
  1. Every `reload_config()` recreates `HotkeyService` (`:462-464`) and leaks any still-blocked or tearing-down thread handle without joining.
  2. When `AudioEngine::drop()` exits `Worker::run`, if MMDevAPI still holds a transient ref to a COM callback on another RPC thread, `audio_tx` stays alive briefly and the detached `ice-forward` thread outlives `AudioEngine::drop()`.
- **Fix:** Replace `forward()` by giving `notify::register`, `VolumeWatch::new`, and `HotkeyService::start` either a direct callback closure `impl Fn(AudioEvent) + Send + 'static` (like `DirWatch::start`) or a `Sender<Msg>` adapter, eliminating all `ice-forward` threads and handles completely.

#### `ENG-06` — `audio_engine.rs` + `resolve.rs`: Duplicate `ChooseDevice` filtering diverges from `resolve.rs` rules
- **Where:** `crates/ice-win/src/audio_engine.rs:618-627` vs `crates/ice-core/src/resolve.rs:137-194`
- **Issue:** When `resolve::resolve()` returns `NeedsConfirmation` or `Ambiguous`, `Worker::follow()` (`audio_engine.rs:618-627`) re-filters `self.endpoints` manually with three bugs compared to `resolve.rs`:
  1. Uses case-sensitive `==` on `hardware_key` / `container_id` (`resolve.rs:142` uses `eq_ignore_ascii_case`).
  2. Does **not** filter Windows' placeholder container `{00000000-0000-0000-FFFF-FFFFFFFFFFFF}` (`resolve.rs:137-139`), so Realtek speakers + Steam Streaming / virtual endpoints sharing the placeholder container can leak into `ChooseDevice.candidates`!
  3. Ignores `DeviceState::Active` (relies on `self.endpoints` only containing active devices) and skips `logical` disambiguation (`Headphones` vs `Headset` hands-free).
- **Fix:** Expose candidate selection directly from `ice_core::resolve` (e.g., include matched `Vec<&'a EndpointInfo>` in `Resolution::Ambiguous` or add `resolve::picker_candidates(dref, endpoints)`) so `resolve.rs` is the single source of truth.

#### `ENG-07` — `audio_engine.rs`: No `VolumeWatch` on capture mic, and same-ID bounce misses stale `VolumeWatch`
- **Where:** `crates/ice-win/src/audio_engine.rs:372, 382-388, 528-534, 578-586`
- **Issue:**
  1. `Worker` creates `volume_watch` only for `(Flow::Render, Role::Multimedia)` (`:383-387`). It never watches `(Flow::Capture, Role::Communications)`, so muting the microphone in Windows Sound settings, a hardware headset boom/button, or Discord never emits `EngineEvent::MicMuteChanged` (stale `snapshot.mic_muted`, breaks F11 persistent red dot).
  2. `if old_default != new_default || self.volume_watch.is_none()` (`:382`) skips re-registering `VolumeWatch` if the default headset powered off and back on with the **same endpoint ID** (exactly the Arctis behavior logged in `planning-discussion.md` §D-16!) or if another driver invalidated the endpoint handle (`AUDCLNT_E_DEVICE_INVALIDATED`, `code-practices.md` §4). Also `AudioEngine` does not refresh `volume_watch` if `AudioEvent::StateChanged` resets the active session.
  3. Notice `HotkeyAction::KnobMic` (`:528-534`) adjusts capture gain silently with no `EngineEvent` emitted for UI feedback.
- **Fix:** Register a second `VolumeWatch` on the default Communications capture device (emitting `MicMuteChanged` / mic-volume event); invalidate and re-create the corresponding `VolumeWatch` whenever a `StateChanged` / `DeviceRemoved` / `DefaultChanged` touches that endpoint ID or a `volume::get`/`set_*` COM call fails.

#### `ENG-08` — `audio_engine.rs`: `startup` notice replay conflicts with D-03 revision checks and never clears
- **Where:** `crates/ice-win/src/audio_engine.rs:48, 102-110, 320-326, 464-475`
- **Issue:**
  1. `startup` stores early notices at `revision = 1..k`. When a UI subscribes later after volume/switch events bumped `snapshot.revision` to e.g. `15`, `subscribe()` replays `revision = 1` envelopes, which compliant D-03 consumers (`env.revision > snapshot.revision`) immediately drop as stale (`native_dot.rs`, `code-practices.md` §6)!
  2. Conversely, any subscriber that doesn't filter by revision sees stale startup notices (e.g., an already-fixed `ConfigReplaced` or resolved `HotkeyConflict`) every time it re-subscribes after `Lite`/gaming mode wakes.
- **Fix:** Move persistent/active notices into `EngineSnapshot` (clearing resolved `HotkeyConflict` / `ConfigInvalid` on `reload_config()`), or assign fresh revisions rather than replaying historical startup revisions out of order.

#### `ENG-09` — `audio_engine.rs` + `file_watch.rs`: `refresh_at` debounce isn't sliding; `DirWatch` handle leaks on callback panic
- **Where:** `crates/ice-win/src/audio_engine.rs:276-278, 444-446` and `crates/ice-win/src/file_watch.rs:45-57`
- **Issue:**
  1. `self.refresh_at.get_or_insert_with(...)` (`audio_engine.rs:445`) schedules 120 ms after the **first** event rather than sliding the deadline like `reload_at` (`:277`). A slow USB hub/Bluetooth arrival burst spanning > 120 ms triggers partial mid-burst enumeration and duplicate `DevicesChanged` churn.
  2. In `file_watch.rs:53-56`, `FindCloseChangeNotification(handles[0])` runs outside an RAII `Drop` guard, leaking the kernel notification handle if `on_change()` unwinds; additionally, if `FindNextChangeNotification` fails (e.g., config folder temporarily removed/recreated), the watcher thread exits silently with zero notice/restart.
  3. `file_watch.rs:48`: `WaitForMultipleObjects` returns `WAIT_OBJECT_0 + 1` cleanly when `stop` is signaled, which works, though using `WAIT_OBJECT_0` check + RAII guard makes error/abort paths explicit.
- **Fix:** Update `self.refresh_at = Some(Instant::now() + REFRESH_DEBOUNCE);` unconditionally (sliding quiet window), and wrap `handles[0]` in a small `Drop` struct in `file_watch.rs`.

#### `ENG-10` — `audio_engine.rs`: `persist_relink()` misses sibling bindings, `favourites`, and case-insensitive IDs
- **Where:** `crates/ice-win/src/audio_engine.rs:707-727`
- **Issue:** `persist_relink()` updates only the single triggered `Binding`/`TogglePair` and entries in `self.config.devices` matching `p.device.endpoint_id == old.endpoint_id` (case-sensitive). Any sibling bindings, the other toggle pairs, or `self.config.favourites.playback/recording` entries pointing at the same `old.endpoint_id` keep stale IDs (requiring redundant fallback scoring on next activation).
- **Fix:** Update all `bindings`, `toggle_pairs`, `favourites`, and `devices` entries whose `endpoint_id.eq_ignore_ascii_case(&old.endpoint_id)` once a switch confirms the re-link.

#### `ENG-11` — `resolve.rs`: `([one], _)` auto-relinks even when uncertain hardware candidates exist
- **Where:** `crates/ice-core/src/resolve.rs:188-193`
- **Issue:** `match (confirmed.as_slice(), uncertain.as_slice()) { ([one], _) => Resolution::Relinked(one), ... }` automatically re-links whenever one candidate matches logically, **ignoring** any `uncertain` candidate (for example, a second identical headset on the same model whose driver didn't populate `form_factor` yet). Per D-16's safety rule, ambiguity must never auto-guess.
- **Fix:** Match `([one], []) => Resolution::Relinked(one)` so `([one], [_, ..])` falls through to `Resolution::Ambiguous`.

---

### Low

#### `ENG-12` — `audio_engine.rs` vs `engine.rs`: Unbounded subscriber channels & `revision` initial value mismatch
- **Where:** `crates/ice-win/src/audio_engine.rs:103, 228, 249` vs `crates/ice-core/src/engine.rs:300, 419`
- **Issue:** Subscriber channels use unbounded `mpsc::channel()` (`audio_engine.rs:103`) — a leaked/stalled subscriber that never disconnects accumulates cloned `DeviceSummary` vectors (`DevicesChanged`) indefinitely. Also, `AudioEngine` starts at `revision = 0` when `startup` has no notices (`:228, 249` skips bumping revision on initial `refresh(false)`), whereas `FakeEngine` initializes `revision = 1` (`engine.rs:300`).
- **Fix:** Align initial `revision = 1` after initial `refresh(false)`, and consider bounded `sync_channel` with `try_send` dropping slow/dead subscribers to force a snapshot resync on overflow (`code-practices.md` §6).

#### `ENG-13` — `audio_engine.rs`: `Console` vs `Multimedia` role split & silent failure drops without `tracing`
- **Where:** `crates/ice-win/src/audio_engine.rs:214-221, 345, 355, 359, 497, 560, 652`
- **Issue:**
  1. `summary().is_default`, `TogglePair`, and `cycle()` query `Role::Console` (`:345, 560, 652`), while `refresh()` tracks `old_default`/`new_default` and `volume_watch` on `Role::Multimedia` (`:359, 385`). Under `RoleSet::Communications` or split-default Windows setups, UI default indicators and volume target can disagree.
  2. `DirWatch::start` (`:220`), `HotkeyService::start` (`:497`), and `eps.list` (`:355`) swallow `Err` silently with no diagnostic log (`code-practices.md` §4 requires structured `tracing` logs).
- **Fix:** Emit `tracing::warn!` / `tracing::error!` on watcher/hotkey/enumeration failures (matching §4), and document or unify `Role::Console` + `Role::Multimedia` checks cleanly.

#### `ENG-14` — `engine.rs` (`FakeEngine`): `simulate_unplug` leaves stale `is_default` / `spatial` on `DeviceLeft`
- **Where:** `crates/ice-core/src/engine.rs:320-326` vs `crates/ice-win/src/audio_engine.rs:397-402`
- **Issue:** `AudioEngine` clears `is_default`, `is_default_communications`, and `spatial: None` on `EngineEvent::DeviceLeft`, whereas `FakeEngine::simulate_unplug` only sets `dev.state = DeviceState::NotPresent`, leaving `is_default = true` and `spatial = Some(...)` in test events (minor parity drift with the real engine).

---

## 3. Recommended Fix Order

1. **`ENG-01` + `ENG-02`** first (eliminates `ice-audio` hang/100%-CPU risks in `hotkeys.rs`, cuts redundant WinRT `IsSpatialAudioFormatSupported` queries during every `refresh()`/`switch()`, and bounds async WinRT `.join()`).
2. **`ENG-03` + `ENG-04`** (fixes `PendingChoice` index corruption on hot-reload and hardens `config.toml` saving against Windows file-sharing errors & external-edit clobbering).
3. **`ENG-05`, `ENG-07`, `ENG-09`** (removes `ice-forward` thread leaks, adds capture mic `VolumeWatch` + same-ID bounce recovery, makes device-burst debounce a true quiet window, and adds RAII handle guard in `DirWatch`).
4. **`ENG-06`, `ENG-08`, `ENG-10`, `ENG-11`** (aligns `resolve.rs` candidate logic & uncertain-match guard, fixes startup notice D-03 revisions, completes sibling re-linking).
