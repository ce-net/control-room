// The ui/src rules of decl.lint, until decl.lint exists: for the hand-written view files of every cell of the site
// (ui/src of each cell, and this repo's sim): no `any`, no `as` casts other than `as const`, no fetch /
// XMLHttpRequest / WebSocket / eval (a view reaches only its own cell, through ui/gen), imports only from ../gen,
// @ui/* and the cells' own src, and no compare against a right or role (a view is never told the viewer's rights).
// The rights a view needs are checked by `cargo build` of its contract (ui-part `views!`). Exit 1 on any finding.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { CELLS } from './cells.mjs';

const ui = resolve(new URL('..', import.meta.url).pathname);
const files = [];
const walk = (d) => { for (const f of readdirSync(d)) { const p = join(d, f); if (statSync(p).isDirectory()) walk(p); else if (p.endsWith('.ts')) files.push(p); } };
for (const c of CELLS) walk(join(c.repo, 'ui/src'));
walk(join(ui, 'sim'));
const bad = [];
const rules = [
  [/:\s*any\b|<any>|\bas any\b/, 'any'],
  [/\bas\s+(?!const\b)[A-Za-z{[(]/, 'as cast'],
  [/\bfetch\s*\(|XMLHttpRequest|new\s+WebSocket|\beval\s*\(|new\s+Function\s*\(/, 'raw network or eval'],
  [/\b(role|rights?|needs)\s*(===|!==|==|!=)|\.(includes|startsWith)\(\s*''\s*\)\s*&&\s*\b(role|rights?)\b/, 'a view compares rights or roles'],
];
for (const f of files) {
  const raw = readFileSync(f, 'utf8');
  for (const m of raw.matchAll(/^import[^'"]*['"]([^'"]+)['"]/gm)) {
    const from = m[1];
    const ok = from.startsWith('../gen/') || from.startsWith('@ui/') || (f.includes('/sim/') && (from.startsWith('../') || from.startsWith('../../')));
    if (!ok) bad.push(`${relative(ui, f)}: imports ${from} (a view imports only ../gen and @ui/*)`);
  }
  const src = raw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '').replace(/'[^'\n]*'|`[^`]*`|"[^"\n]*"/g, "''");
  src.split('\n').forEach((l, i) => { const code = /^\s*(import|export)\s.*\{/.test(l) ? l.replace(/\{[^}]*\}/, '{}') : l; for (const [re, what] of rules) if (re.test(code)) bad.push(`${relative(ui, f)}:${i + 1}: ${what}: ${l.trim()}`); });
}
if (bad.length) { console.error(bad.join('\n')); process.exit(1); }
console.log(`lint: ${files.length} view files clean`);
