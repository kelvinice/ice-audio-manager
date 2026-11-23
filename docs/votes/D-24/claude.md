# D-24 · Claude's vote

> Sealed at proposal time (§0.4 rule 2). Claude wrote the entry; it contains no scores or verdicts.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| G0 · Stay MIT, clean-room only | +1 | Everything we've wanted so far (LRCLIB lookups, lyrics UI, design) can be written from docs and observation |
| G1 · GPL-3.0 now | −1 | Pays the irreversible cost before we've named a single GPL component worth porting |
| G2 · MIT now, relicense per component | +2 | Keeps the owner's offer available, triggered by a concrete, policy-clean component |

## Strongest argument against my top choice (G2)

Deferring has a cost. If a feature is built clean-room under MIT and we relicense later, that work was done twice. And "per
component" invites drift: each time someone wants a GPL snippet, the licence question reopens. Deciding GPL now (G1) would
settle it once and let agents port freely, which the owner explicitly said they're fine with.

## New option or amendment

G2's trigger, written into code-practices §1 or §9 if adopted:
1. A named GPL component.
2. One paragraph on why writing our own is clearly worse.
3. A check that it doesn't fall under D-22 B3/K3 (unofficial YouTube APIs) or §1.
4. A new D-xx with a blind vote.

Until then, agents may *read* GPL references and must not copy or port them.

## Evidence I checked myself

- `git log --format="%an <%ae>" | sort | uniq -c`: 91 `kelvinice` + 9 `Kelvin`, the same address. No third-party authors.
- `cargo metadata`: every crate is MIT, Apache-2.0, BSD-3-Clause, Zlib, ISC, Unicode-3.0, Unlicense, 0BSD, CC0 or MPL-2.0
  (7). All are GPL-3.0-compatible. `app/ui/package.json` dependencies are MIT.
- `LICENSE`: "MIT License, Copyright (c) 2026 kelvinice".
- The GPL candidates we have (BitChord, Echo-Music: Kotlin; ytm-rs: Rust). Their distinctive parts are InnerTube and stream
  extraction, which D-22 B3 covers whatever the licence. The lyrics client logic is a few HTTP calls documented by LRCLIB.

## Position

The owner's offer is valuable and should stay open, but relicensing buys nothing until there's a specific GPL component we
can't reasonably write ourselves *and* that passes D-22/§1. G2 keeps both doors open.

## Revision round (after reveal, 2026-11-23)

No score changes. I back both amendments:
- Gemini's clean-room rule. GPL/AGPL references are used only for screenshots and public protocol facts, never for internal
  helper algorithms before writing an MIT equivalent. I note I read `LrcLib.kt`'s duration tolerances myself (D-22 facts).
  The tolerances are behaviour facts, but its title-cleaning regexes must be written independently from LRCLIB docs and our
  own test cases.
- Codex's import record for G2: upstream revision, exact licence grant (GPL-3.0-only vs -or-later), files, and the ongoing
  cost of keeping a port current.
