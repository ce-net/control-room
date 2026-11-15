// Live proof driver: open the control room as one person in headless Chromium, the way the apps.ce-net.com gate
// would send them (the gate's signed X-Ce-Person headers, made here with this host's gate person key), record every
// byte the browser receives (HTTP bodies and socket frames), take screenshots, and search the bytes for what this
// viewer must not receive.
// Usage: node tools/prove.mjs <person> <out-dir> [--base http://127.0.0.1:8834/] [--forbid a,b,c] [--watch-ms N]
//        [--tab devices] [--width 1280 --height 900]
import puppeteer from 'puppeteer-core';
import { createHmac } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : d; };
const [person, out] = args;
const base = opt('--base', 'http://127.0.0.1:8834/');
const forbid = (opt('--forbid', '') || '').split(',').filter(Boolean);
const watchMs = Number(opt('--watch-ms', '6000'));
const width = Number(opt('--width', '1280'));
const height = Number(opt('--height', '900'));
const tabs = (opt('--tabs', '') || '').split(',').filter(Boolean);
// --during "<cmd>": run once the page is up ($LINK is the minted link); --timeline-ms N: then record the page every 2 s
const during = opt('--during', '');
const timelineMs = Number(opt('--timeline-ms', '0'));
// --tamper <sha256 prefix>: that bundle reaches the browser with one byte changed (the built file, byte 100 xor 1)
const tamper = opt('--tamper', '');
mkdirSync(out, { recursive: true });

const key = Buffer.from(readFileSync(process.env.CE_GATE_PERSON_KEY ?? `${process.env.HOME}/.config/work-show/person-key`, 'utf8').trim(), 'hex');
const signed = () => {
  const at = String(Math.floor(Date.now() / 1000));
  const sig = createHmac('sha256', key).update(`ce-person\n${person}\n${at}`).digest('hex');
  return { 'X-Ce-Person': person, 'X-Ce-Person-At': at, 'X-Ce-Person-Sig': sig };
};
const link = execFileSync(`${process.env.HOME}/.local/bin/cr-shell`, ['mint', person, base], { stdio: ['ignore', 'pipe', 'inherit'] }).toString().trim();

const browser = await puppeteer.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox', '--disable-gpu'] });
const page = await browser.newPage();
await page.setViewport({ width, height, deviceScaleFactor: 1 });
await page.setExtraHTTPHeaders(signed());
const refresh = setInterval(() => void page.setExtraHTTPHeaders(signed()), 20_000);
if (tamper !== '') {
  await page.setRequestInterception(true);
  page.on('request', (rq) => {
    if (rq.url().includes(`/_c/${tamper}`)) {
      const file = rq.url().split('/_c/')[1].split('?')[0];
      const b = Buffer.from(readFileSync(join(new URL('..', import.meta.url).pathname, 'dist/_c', file)));
      b[100] ^= 1;
      void rq.respond({ status: 200, contentType: 'text/javascript', body: b });
    } else void rq.continue();
  });
}
const cdp = await page.createCDPSession();
await cdp.send('Network.enable');
const received = [];
const http = [];
let frames = 0;
cdp.on('Network.webSocketFrameReceived', (e) => { frames += 1; received.push(Buffer.from(e.response.payloadData, e.response.opcode === 2 ? 'base64' : 'utf8')); });
page.on('response', async (r) => {
  try { const b = await r.buffer(); received.push(b); http.push({ url: r.url().replace(/#.*/, ''), status: r.status(), bytes: b.length }); } catch { /* redirects have no body */ }
});
const consoleLines = [];
page.on('console', (m) => consoleLines.push(m.text()));
await page.goto(link, { waitUntil: 'networkidle2' });
await new Promise((r) => setTimeout(r, watchMs));
const shot = async (name) => { await page.screenshot({ path: join(out, `${name}.png`), fullPage: false }); return `${name}.png`; };
const shots = [await shot(`${person}-${width}`)];
for (const t of tabs) {
  const b = await page.$(`nav.rail button[data-slot="${t}"]`);
  if (b) { await b.click(); await new Promise((r) => setTimeout(r, 2500)); shots.push(await shot(`${person}-${width}-${t}`)); }
}
const look = () => page.evaluate(() => ({
  tabs: Array.from(document.querySelectorAll('nav.rail button')).map((b) => b.getAttribute('data-slot')),
  slots: Array.from(document.querySelectorAll('.slot')).map((s) => s.getAttribute('data-state')),
  devices: Array.from(document.querySelectorAll('[data-device]')).map((d) => `${d.getAttribute('data-device')}:${d.getAttribute('data-online')}`),
  faces: Array.from(document.querySelectorAll('.face')).map((f) => f.textContent),
}));
const timeline = [];
if (during !== '') {
  const t0 = Date.now();
  const r = execFileSync('/bin/sh', ['-c', during], { env: { ...process.env, LINK: link }, stdio: ['ignore', 'pipe', 'pipe'] }).toString().trim();
  timeline.push({ t_ms: 0, ran: during, out: r.slice(0, 400) });
  while (Date.now() - t0 < timelineMs) {
    await new Promise((res) => setTimeout(res, 2000));
    timeline.push({ t_ms: Date.now() - t0, ...(await look()) });
  }
  shots.push(await shot(`${person}-${width}-after`));
}
const state = await page.evaluate(() => ({
  tabs: Array.from(document.querySelectorAll('nav.rail button')).map((b) => b.getAttribute('data-slot')),
  who: document.querySelector('.who')?.textContent ?? '',
  devices: Array.from(document.querySelectorAll('[data-device]')).map((d) => `${d.getAttribute('data-device')}:${d.getAttribute('data-online')}`),
  keyRows: document.querySelectorAll('[data-view="KeyList"] tbody tr').length,
  auditRows: document.querySelectorAll('[data-view="AuditTrail"] tbody tr').length,
  url: location.href,
}));
const all = Buffer.concat(received);
const found = Object.fromEntries(forbid.map((f) => [f, all.includes(Buffer.from(f))]));
const result = { person, width, http, frames, bytes: all.length, state, forbidden_found: found, shots, timeline, console: consoleLines.slice(0, 20) };
writeFileSync(join(out, `${person}-${width}.json`), JSON.stringify(result, null, 2));
writeFileSync(join(out, `${person}-${width}.bytes`), all);
console.log(JSON.stringify(result));
clearInterval(refresh);
await browser.close();
