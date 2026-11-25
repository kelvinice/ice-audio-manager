import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// A token declared only inside one `[data-theme=…]` block is undefined on every other theme, so a `var()` using it
// falls back to the property's initial value. That made the D-27 plain notch transparent (invisible) on all themes
// but Minimal. Every theme block may only override tokens that `:root` already declares.

const css = readFileSync(new URL('./base.css', import.meta.url), 'utf8');

function declaredTokens(selector: (sel: string) => boolean): Map<string, Set<string>> {
  const out = new Map<string, Set<string>>();
  for (const [, rawSel, body] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const sel = rawSel.replace(/\/\*[\s\S]*?\*\//g, '').trim();
    if (!selector(sel)) continue;
    const tokens = out.get(sel) ?? new Set<string>();
    for (const [, name] of body.matchAll(/(--ice-[\w-]+)\s*:/g)) tokens.add(name);
    out.set(sel, tokens);
  }
  return out;
}

test('every token a theme overrides is declared in :root', () => {
  const root: Set<string> = declaredTokens((sel) => sel === ':root').get(':root') ?? new Set();
  assert.ok(root.size > 0, ':root block found');
  const themes = declaredTokens((sel) => sel.startsWith('[data-theme='));
  assert.ok(themes.size >= 4, 'theme blocks found');
  for (const [theme, tokens] of themes) {
    const missing: string[] = [...tokens].filter((t) => !root.has(t));
    assert.deepEqual(missing, [], `${theme} declares tokens :root lacks`);
  }
});

test('the hibernate tokens are global, so the plain notch is drawn on every theme', () => {
  const root = declaredTokens((sel) => sel === ':root').get(':root');
  assert.ok(root?.has('--ice-hibernate-bg'));
  assert.ok(root?.has('--ice-hibernate-dot'));
});
