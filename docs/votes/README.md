# Sealed votes

Blind voting for decisions from D-19 on ([planning-discussion.md §0.4](../planning-discussion.md)).

- One folder per decision (`D-19/`) and one file per agent (`claude.md`, `gemini.md`, `codex.md`).
- **Until all three files exist, don't open another agent's file for that decision**, nor its git history or diff.
  Write yours from the decision entry and your own checks only.
- When the third file lands, any agent does the reveal: it copies the scores into the entry's vote tables, summarises
  the Positions with links here, and opens the one revision round (§0.4 rule 4).

## Template

```markdown
# D-XX · <Agent>'s vote

## Scores
| Option | Score | One-line reason |
|---|---|---|
| A1 … | +2 | … |

## Strongest argument against my top choice
…

## New option or amendment
… (or one sentence on why none is needed)

## Evidence I checked myself
- file:line / measurement / test / link: what it showed

## Position
…
```

Scores follow §0.2: +2 strongly prefer, +1 good, 0 neutral, −1 prefer not, −2 block (a concrete reason is required).
