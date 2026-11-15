// ControlRoom, the face of cell:cr.shell@2: the title, who the page is for, and one tab per slot cell. The tabs come
// from the shell's own backend (`Layout`), which names only the slots this viewer may open, so this bundle carries no
// slot name at all. A slot whose panel leaves (its grant ended or was revoked) loses its tab.
import { ControlRoom } from '../gen/CrShell.ui';
import type { Tab } from '../gen/CrShell';
import { clear, date, h } from '@ui/kit/dom';
import { failed, loading } from '@ui/kit/faces';

function theme(): HTMLElement {
  const root = document.documentElement;
  const b = h('button', { class: 'btn quiet', label: 'Switch light and dark' });
  const draw = (): void => { b.textContent = root.dataset['theme'] === 'dark' ? 'Light' : 'Dark'; };
  b.addEventListener('click', () => {
    root.dataset['theme'] = root.dataset['theme'] === 'dark' ? 'light' : 'dark';
    try { localStorage.setItem('ce-theme', root.dataset['theme']); } catch { /* private window */ }
    draw();
  });
  draw();
  return b;
}

export default ControlRoom.render((v) => {
  const rail = h('nav', { class: 'rail', label: 'Panels' });
  const crumb = h('div', { class: 'crumb' });
  const who = h('div', { class: 'who mono' });
  const stage = h('main', { class: 'main' }, loading());
  let open = '';
  let tabs: readonly Tab[] = [];
  const gone = new Set<string>();
  const show = (t: Tab): void => {
    open = t.slot;
    for (const b of Array.from(rail.querySelectorAll('button'))) b.setAttribute('aria-pressed', String(b.dataset['slot'] === t.slot));
    clear(crumb, 'ce-net / ', h('b', { text: 'Control room' }), ` / ${t.label}`);
    try { localStorage.setItem('cr-tab', t.slot); } catch { /* private window */ }
    const el = v.place(t.slot);
    if (!el.isConnected || el.parentElement !== stage) clear(stage, el);
  };
  const drawRail = (): void => {
    const live = tabs.filter((t) => !gone.has(t.slot));
    clear(rail, ...live.map((t) => h('button', { pressed: t.slot === open, attrs: { 'data-slot': t.slot }, onclick: () => show(t) }, h('span', { class: 'dot' }), t.label)));
    if (live.length === 0) return clear(stage, h('div', { class: 'face', text: 'No panel is open to your sign-in.' }));
    const cur = live.find((t) => t.slot === open);
    if (cur === undefined) {
      let last = '';
      try { last = localStorage.getItem('cr-tab') ?? ''; } catch { /* private window */ }
      const first = live.find((t) => t.slot === last) ?? live[0];
      if (first !== undefined) show(first);
    }
  };
  // the one thing a parent can know about a slot: whether something is mounted in it
  const watchSlot = (t: Tab): void => {
    const el = v.place(t.slot);
    new MutationObserver(() => {
      if (el.dataset['state'] === 'empty' && !gone.has(t.slot)) { gone.add(t.slot); if (open === t.slot) open = ''; drawRail(); }
    }).observe(el, { attributes: true, attributeFilter: ['data-state'] });
  };
  v.draw((d, why) => {
    if (d.layout === undefined) return clear(stage, why === null ? loading() : failed(why));
    const l = d.layout;
    who.textContent = `${l.person} · ${l.role}${l.until_ms > 0n ? ' · until ' + date(l.until_ms) : ''}`;
    const fresh = l.tabs.map((t) => t.slot).join(',') !== tabs.map((t) => t.slot).join(',');
    if (fresh) {
      for (const t of l.tabs) if (!tabs.some((x) => x.slot === t.slot)) watchSlot(t);
      tabs = l.tabs;
      drawRail();
    }
  });
  let t = 'light';
  try { t = localStorage.getItem('ce-theme') === 'dark' ? 'dark' : 'light'; } catch { /* private window */ }
  document.documentElement.dataset['theme'] = t;
  return h(
    'div',
    { class: 'shell' },
    rail,
    h('div', { class: 'col' }, h('header', { class: 'top' }, h('div', {}, crumb, h('h1', { class: 'display', text: 'Control room.' })), h('nav', { class: 'btns glass' }, who, theme())), stage),
  );
});
