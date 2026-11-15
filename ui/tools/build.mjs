// The site build (ui.bundle, minimum): for every cell of the site, the ts-web client (ce-wire-gen) and the view
// bindings (ui-gen) into that cell's ui/gen; then `tsc --strict` over every view at once (a renamed contract field
// fails here, in the view that uses it); then one ES module per view, named by its sha256 (dist/_c/<sha256>.js) and
// listed in dist/bundles.json, which the gateway reads and never serves; then the page (index.html, the host module
// and the kit under _k/). The public sim page is built from sim/main.ts into sim-dist/, with fake data only.
// Usage: node tools/build.mjs [--check]   (--check: generate and type-check only)
import { build } from 'esbuild';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ui = join(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(ui, '..');
const dev = resolve(repo, '..');
const uiCells = process.env.UI_CELLS ?? join(dev, 'ui-cells');
const sh = (cmd, args, cwd) => execFileSync(cmd, args, { cwd, stdio: ['ignore', 'pipe', 'inherit'] }).toString();

import { CELLS } from './cells.mjs';

// 1. codegen
sh('cargo', ['build', '-q', '-p', 'ui-gen'], uiCells);
const uiGen = join(uiCells, 'target/debug/ui-gen');
for (const c of CELLS) {
  const gen = join(c.repo, 'ui/gen');
  mkdirSync(gen, { recursive: true });
  sh('ce-wire-gen', ['--lang', 'ts-web', c.contract, '-o', 'ui/gen', '--api', c.ident], c.repo);
  for (const extra of ['ts', 'web']) rmSync(join(gen, extra), { recursive: true, force: true });
  sh(uiGen, [c.contract, '--client', `./${c.ident}`, '--rt', '@ui/rt', '-o', `ui/gen/${c.ident}.ui.ts`], c.repo);
}

// 2. one strict type-check over the host, every view and the sim
const paths = { '@ui/rt': [join(uiCells, 'ts/ui-rt.ts')], '@ui/host': [join(uiCells, 'ts/ui-host.ts')], '@ui/kit/*': [join(uiCells, 'ts/kit/*')] };
const tsconfig = {
  compilerOptions: {
    target: 'ES2022', module: 'ESNext', moduleResolution: 'Bundler', lib: ['ES2022', 'DOM', 'DOM.Iterable'],
    strict: true, noImplicitReturns: true, noFallthroughCasesInSwitch: true,
    useUnknownInCatchVariables: true, noEmit: true, skipLibCheck: true, isolatedModules: true, paths,
  },
  files: [join(ui, 'src/host.ts'), join(ui, 'sim/main.ts'), ...CELLS.map((c) => join(c.repo, c.view))],
};
writeFileSync(join(ui, 'tsconfig.build.json'), JSON.stringify(tsconfig, null, 2) + '\n');
try {
  sh(join(ui, 'node_modules/.bin/tsc'), ['-p', join(ui, 'tsconfig.build.json')], ui);
} catch (e) {
  process.stdout.write(e.stdout?.toString() ?? '');
  console.error('tsc: the views do not type-check against their contracts');
  process.exit(1);
}
if (process.argv.includes('--check')) {
  console.log(JSON.stringify({ check: 'ok', cells: CELLS.map((c) => c.key) }));
  process.exit(0);
}

// 3. bundles, by digest
const alias = { '@ui/rt': paths['@ui/rt'][0], '@ui/host': paths['@ui/host'][0], '@ui/kit': join(uiCells, 'ts/kit') };
const common = { bundle: true, format: 'esm', target: 'es2022', minify: true, legalComments: 'none', alias, logLevel: 'warning', write: false };
const sha = (b) => createHash('sha256').update(b).digest('hex');
const sri = (b) => 'sha384-' + createHash('sha384').update(b).digest('base64');
const out = join(ui, 'dist');
rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, '_c'), { recursive: true });
mkdirSync(join(out, '_k'), { recursive: true });
const bundles = {};
for (const c of CELLS) {
  const r = await build({ ...common, entryPoints: [join(c.repo, c.view)] });
  const bytes = r.outputFiles[0].contents;
  const h = sha(bytes);
  writeFileSync(join(out, '_c', `${h}.js`), bytes);
  bundles[c.key] = { file: `_c/${h}.js`, sha256: h };
}
writeFileSync(join(out, 'bundles.json'), JSON.stringify(bundles, null, 2) + '\n');

// 4. the page: the same for every viewer, naming no cell but the shell's own client
function kit(dir) {
  const css = ['tokens.css', 'components.css', 'app.css'].map((f) => readFileSync(join(uiCells, 'ts/kit', f), 'utf8')).join('\n') + readFileSync(join(ui, 'src/page.css'), 'utf8');
  const name = `app.${sha(css).slice(0, 12)}.css`;
  writeFileSync(join(dir, name), css);
  cpSync(join(uiCells, 'ts/kit/fonts'), join(dir, 'fonts'), { recursive: true });
  return [name, Buffer.from(css)];
}
function page(title, script, scriptBytes, style, styleBytes, base) {
  return `<!doctype html>
<html lang="en" data-theme="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<meta name="theme-color" content="#f6f7f4">
<title>${title}</title>
<link rel="stylesheet" href="${base}${style}" integrity="${sri(styleBytes)}">
<script type="module" src="${base}${script}" integrity="${sri(scriptBytes)}"></script>
</head>
<body><div id="app"></div></body>
</html>
`;
}
const host = (await build({ ...common, entryPoints: [join(ui, 'src/host.ts')] })).outputFiles[0].contents;
const hostName = `host.${sha(host).slice(0, 12)}.js`;
writeFileSync(join(out, '_k', hostName), host);
const [style, styleBytes] = kit(join(out, '_k'));
writeFileSync(join(out, 'index.html'), page('Control room', hostName, host, style, styleBytes, '_k/'));

// 5. the public sim page: the same views, fake sources, no grant, no socket
const sim = join(ui, 'sim-dist');
rmSync(sim, { recursive: true, force: true });
mkdirSync(sim, { recursive: true });
const simJs = (await build({ ...common, entryPoints: [join(ui, 'sim/main.ts')] })).outputFiles[0].contents;
const simName = `sim.${sha(simJs).slice(0, 12)}.js`;
writeFileSync(join(sim, simName), simJs);
const [simCss, simCssBytes] = kit(sim);
writeFileSync(join(sim, 'index.html'), page('Control room (sim)', simName, simJs, simCss, simCssBytes, ''));
const kb = (b) => Math.round(b.length / 102.4) / 10;
console.log(JSON.stringify({ bundles: Object.fromEntries(Object.entries(bundles).map(([k, v]) => [k, v.file])), host: hostName, sim: simName, kB: Object.fromEntries(CELLS.map((c) => [c.key, kb(readFileSync(join(out, bundles[c.key].file)))])), from: relative(dev, repo) }));
if (!existsSync(join(out, 'index.html'))) process.exit(1);

// the public page is served from docs/ on main (GitHub Pages): the sim and nothing of the live system
const docs = join(repo, 'docs');
for (const f of readdirSync(docs)) if (/^(sim\.[0-9a-f]+\.js|app\.[0-9a-f]+\.css|index\.html)$/.test(f)) rmSync(join(docs, f));
rmSync(join(docs, 'fonts'), { recursive: true, force: true });
cpSync(sim, docs, { recursive: true });
writeFileSync(join(docs, '.nojekyll'), '');
