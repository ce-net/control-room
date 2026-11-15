// The public sim page: the same four views as the live page, mounted with sim sources of the same generated types.
// Every row is invented in this browser; nothing is read from any mesh, key store or device. A switch shows the page
// as root and as an operator, with the server's rules applied here in the sim the way the cells apply them live.
import type { ControlRoomSource } from '../gen/CrShell.ui';
import type { KeyRow, KeyDelta, RotationState } from '../../../cr-keys/ui/gen/CrKeys';
import type { DeviceRow, DeviceDelta } from '../../../cr-devices/ui/gen/CrDevices';
import type { AuditRow, AuditDelta } from '../../../cr-audit/ui/gen/CrAudit';
import type { KeyListSource } from '../../../cr-keys/ui/gen/CrKeys.ui';
import type { DeviceListSource } from '../../../cr-devices/ui/gen/CrDevices.ui';
import type { AuditTrailSource } from '../../../cr-audit/ui/gen/CrAudit.ui';
import type { Feed, Host } from '@ui/rt';
import ControlRoom from '../src/ControlRoom';
import KeyList from '../../../cr-keys/ui/src/KeyList';
import DeviceList from '../../../cr-devices/ui/src/DeviceList';
import AuditTrail from '../../../cr-audit/ui/src/AuditTrail';
import { h } from '@ui/kit/dom';

type Viewer = 'root' | 'operator';

let seed = 7;
const rnd = (): number => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; };
function pick<T>(xs: readonly T[]): T {
  const x = xs[Math.floor(rnd() * xs.length)];
  if (x === undefined) throw new Error('empty pick');
  return x;
}
const hex = (n: number): string => Array.from({ length: n }, () => Math.floor(rnd() * 16).toString(16)).join('');
const DAY = 86400;
const nowS = (): number => Math.floor(Date.now() / 1000);

const SERVICES = ['payments', 'mail', 'git', 'maps', 'storage', 'search', 'sms', 'cdn'];
const KEYS: KeyRow[] = Array.from({ length: 24 }, (_, i) => {
  const service = pick(SERVICES);
  const left = Math.round(rnd() * 120) - 10;
  const rotation: RotationState = left < 0 ? { tag: 'Overdue', days: -left } : left < 30 ? { tag: 'DueSoon', days: left } : { tag: 'Fresh' };
  const t = nowS();
  return {
    name: `SIM_${service.toUpperCase()}_${['API_KEY', 'TOKEN', 'WEBHOOK_SECRET'][i % 3] ?? 'KEY'}_${i}`,
    service, purpose: `${service} for the sim shop`, owner: pick(['billing', 'ops', 'web']),
    version: BigInt(1 + Math.floor(rnd() * 6)), sha256_8: hex(8), written_at: BigInt(t - 40 * DAY),
    expires_at: BigInt(t + left * DAY), days_left: BigInt(left), rotation, self_renewing: rnd() > 0.3,
    last_check: rnd() > 0.1 ? 'ok' : 'fail: 401', last_check_at: BigInt(t - 3600),
    holders: [{ name: 'sim-node-a', home: hex(64), lanes: ['*'], not_after: BigInt(t + left * DAY) }],
    last_opened: BigInt(t - Math.floor(rnd() * 9 * DAY)), refs: [],
  };
});

const NODES = ['sim-ops-1', 'sim-desk', 'sim-laptop', 'sim-phone', 'sim-server-2'];
const DEVICES: DeviceRow[] = NODES.map((name, i) => ({
  id: hex(64), name, kind: name.includes('phone') ? { tag: 'Phone' } : name.includes('laptop') ? { tag: 'Laptop' } : name.includes('desk') ? { tag: 'Desktop' } : { tag: 'Server' },
  addrs: [`10.0.0.${10 + i}`], online: i !== 4, last_seen_ms: BigInt(Date.now() - (i === 4 ? 3 * DAY * 1000 : 0)), checked_ms: BigInt(Date.now()),
  rtt_ms: i === 4 ? null : 2 + Math.floor(rnd() * 40), cells: name.includes('phone') ? null : 20 + Math.floor(rnd() * 300),
  why: i === 4 ? '10.0.0.14: no answer in 4000 ms' : '', inventory: null,
  tailscale: { ip: `100.64.0.${10 + i}`, online: i !== 4, key_expires_at: null }, passkeys: name.includes('phone') ? [{ label: 'sim phone passkey', kind: 'P-256' }] : [],
  refs: [],
}));

const APPS = ['orch', 'access.grant', 'keys', 'cell.install', 'state.store'];
let seq = 1000;
const auditRow = (at: number): AuditRow => {
  const app = pick(APPS);
  const node = pick(NODES.slice(0, 2));
  seq += 1;
  const ok = rnd() > 0.12;
  return {
    id: `${node}:${seq}`, seq: BigInt(seq), at_ms: BigInt(at), node, writer: hex(16), actor: pick(['sim-lane-a', 'sim-lane-b', hex(16)]), app,
    op: pick(app === 'orch' ? ['Up', 'Ps', 'Restart'] : app === 'keys' ? ['Meta', 'List', 'Audit'] : ['List', 'Mint', 'Put']),
    target: `${app}/sim-${hex(4)}`, ok, kind: app === 'keys' ? { tag: 'KeyEvent' } : app === 'access.grant' ? { tag: 'Grant' } : { tag: 'Call' },
    chain_id: rnd() > 0.5 ? hex(16) : null, detail: `sim detail ${hex(6)}`, refs: [],
  };
};
const AUDIT: AuditRow[] = Array.from({ length: 60 }, (_, i) => auditRow(Date.now() - i * 47_000));

/** A sim feed: a reset, then `next()` deltas every `ms`. */
function feed<R, D>(reset: () => D, next: () => D | null, ms: number): Feed<D> & AsyncIterable<D> {
  let closed = false;
  const queue: D[] = [reset()];
  let wake: (() => void) | null = null;
  const id = setInterval(() => { const d = next(); if (d !== null) { queue.push(d); wake?.(); } }, ms);
  return {
    close() { closed = true; clearInterval(id); wake?.(); },
    async *[Symbol.asyncIterator]() {
      while (!closed) {
        const d = queue.shift();
        if (d !== undefined) { yield d; continue; }
        await new Promise<void>((r) => { wake = r; });
        wake = null;
      }
    },
  };
}

// what the cells would let each viewer have (the live cells decide this on the server; the sim mimics them here)
const keysFor = (v: Viewer): KeyRow[] => (v === 'root' ? KEYS : []);
const devicesFor = (v: Viewer): DeviceRow[] => (v === 'root' ? DEVICES : DEVICES.filter((d) => d.name === 'sim-ops-1').map((d) => ({ ...d, tailscale: null, passkeys: null })));
const auditFor = (v: Viewer, rows: AuditRow[]): AuditRow[] => (v === 'root' ? rows : rows.filter((r) => r.app === 'orch').map((r) => ({ ...r, detail: null })));

function sources(v: Viewer): { shell: ControlRoomSource; keys: KeyListSource; devices: DeviceListSource; audit: AuditTrailSource } {
  const live = [{ cell: 'sim', node: 'this browser', state: { tag: 'Live' as const }, ms: 0 }];
  return {
    shell: {
      layout: async () => ({ title: 'Control room', person: v === 'root' ? 'sim-owner' : 'sim-operator', role: v, until_ms: BigInt(Date.now() + 8 * 3600_000), tabs: v === 'root' ? [{ slot: 'keys', label: 'Keys' }, { slot: 'devices', label: 'Devices' }, { slot: 'audit', label: 'Audit' }] : [{ slot: 'devices', label: 'Devices' }, { slot: 'audit', label: 'Audit' }] }),
    },
    keys: {
      sources: async () => live,
      watch: (q) => feed<KeyRow, KeyDelta>(() => ({ reset: true, put: keysFor(v).filter((k) => (q.q === null || k.name.toLowerCase().includes(q.q.toLowerCase())) && (q.due_within_days === null || (k.days_left ?? 999n) <= BigInt(q.due_within_days))), del: [], hlc: 0n }), () => null, 5000),
    },
    devices: {
      sources: async () => live,
      // the sim laptop goes offline and comes back every 20 s, so a row changes by itself
      watch: () => {
        let tick = 0;
        return feed<DeviceRow, DeviceDelta>(() => ({ reset: true, put: devicesFor(v), del: [], hlc: 0n }), () => {
          tick += 1;
          const d = DEVICES[2];
          if (d === undefined) return null;
          const online = tick % 2 === 0;
          const row: DeviceRow = { ...d, online, last_seen_ms: online ? BigInt(Date.now()) : d.last_seen_ms, why: online ? '' : '10.0.0.12: no answer in 4000 ms', rtt_ms: online ? 7 : null };
          DEVICES[2] = row;
          const visible = devicesFor(v).find((x) => x.id === row.id);
          return visible === undefined ? null : { reset: false, put: [visible], del: [], hlc: BigInt(Date.now()) };
        }, 10_000);
      },
    },
    audit: {
      sources: async () => live,
      watch: () => feed<AuditRow, AuditDelta>(() => ({ reset: true, put: auditFor(v, AUDIT), del: [], hlc: 0n }), () => {
        const r = auditRow(Date.now());
        AUDIT.unshift(r);
        const put = auditFor(v, [r]);
        return put.length === 0 ? null : { reset: false, put, del: [], hlc: BigInt(Date.now()) };
      }, 4000),
    },
  };
}

const app = document.getElementById('app');
let stop: () => void = () => {};
function mount(v: Viewer): void {
  if (app === null) return;
  stop();
  const s = sources(v);
  const stage = h('div');
  const bar = h(
    'div',
    { class: 'simbar' },
    'Sim mode: every row on this page is generated in your browser; no mesh, key or device is behind it. View as ',
    h('button', { class: 'btn quiet', pressed: v === 'root', text: 'root', onclick: () => mount('root') }),
    ' ',
    h('button', { class: 'btn quiet', pressed: v === 'operator', text: 'operator', onclick: () => mount('operator') }),
  );
  app.replaceChildren(h('div', { class: 'col' }, bar), stage);
  const stops: (() => void)[] = [];
  const child = (name: string): HTMLElement => {
    const el = h('div', { class: 'slot', attrs: { 'data-state': 'empty' } });
    const quiet: Host<unknown> = { props: {}, emit: () => {}, slot: () => h('div'), gone: () => { el.dataset['state'] = 'empty'; } };
    el.dataset['state'] = 'mounted';
    if (name === 'keys' && v === 'root') stops.push(KeyList.mountWith(el, s.keys, { ...quiet, props: { q: null, due_within_days: null } }));
    else if (name === 'devices') stops.push(DeviceList.mountWith(el, s.devices, { ...quiet, props: { q: null, only_offline: null } }));
    else if (name === 'audit') stops.push(AuditTrail.mountWith(el, s.audit, { ...quiet, props: { q: null, app: null, only_failed: null } }));
    else el.dataset['state'] = 'empty';
    return el;
  };
  stops.push(ControlRoom.mountWith(stage, s.shell, { props: { route: null }, emit: () => {}, slot: child, gone: () => {} }));
  stop = () => { for (const f of stops) f(); };
}
mount('root');
