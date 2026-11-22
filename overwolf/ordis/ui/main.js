// Main window. Everything is computed from the background window's state.
import { answer, HELP } from '../core/assistant.js';
import { closeToCompleting, describe } from '../core/close.js';
import { goalStatus } from '../core/goals.js';
import { neededParts, relicRows, masteryRows, foundryRows, buildableNow } from '../core/insight.js';
import { count } from '../core/account.js';
import { fmtN, fmtDuration } from '../core/util.js';

const TABS = ['Overview', 'Ordis', 'Goals', 'Inventory', 'Relics', 'Mastery', 'Foundry', 'Settings'];
let tab = 'Overview', bg = null, cat = null, chat = [], filter = '';
const $ = (s) => document.querySelector(s);

function h(tag, attrs, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (k.startsWith('on')) e.addEventListener(k.slice(2), v); else if (k === 'class') e.className = v; else if (v !== false && v != null) e.setAttribute(k, v);
  }
  for (const k of kids.flat()) if (k != null && k !== false) e.append(k.nodeType ? k : document.createTextNode(String(k)));
  return e;
}
const ctx = () => ({ cat, acct: bg.state.acct, goals: bg.state.goals });
const ago = (ms) => (ms ? `${fmtDuration(Date.now() - ms)} ago` : 'never');
const card = (l, v) => h('div', { class: 'card' }, h('div', { class: 'v' }, v), h('div', { class: 'l' }, l));
const bar = (f) => h('div', { class: 'bar' }, h('i', { style: `width:${Math.round(Math.max(0, Math.min(1, f)) * 100)}%` }));
const table = (head, rows) => h('table', {}, h('tr', {}, head.map((x) => h('th', {}, x))), rows.map((r) => h('tr', {}, r.map((c) => h('td', {}, c)))));

function pill() {
  const st = bg.state;
  const p = $('#pill');
  let t = 'Waiting for Warframe', c = 'warn';
  if (st.acct) { t = `Inventory ${ago(st.updatedAt)}`; c = 'good'; } else if (st.status.gameRunning) { t = 'Game running, waiting for inventory'; }
  p.textContent = t; p.className = `pill ${c}`;
}

function needData(title) {
  const st = bg.state;
  return h('div', { class: 'empty' }, h('h2', {}, title),
    st.catalogError ? h('p', { class: 'bad' }, `The item catalog failed to load: ${st.catalogError}`) : null,
    !cat ? h('p', {}, 'Loading the item catalog…') : null,
    cat && !st.acct ? [h('p', {}, 'Ordis has not received your inventory yet.'),
      h('p', { class: 'dim' }, 'Start Warframe with Overwolf running. The game sends the inventory when it syncs, which often happens when you leave the Relay or Orbiter. Settings shows exactly what has arrived.')] : null);
}

const views = {
  Overview() {
    const { acct, goals } = ctx();
    const close = closeToCompleting(cat, acct, goals, 5);
    return h('div', {},
      h('h2', {}, acct.mr ? `Mastery rank ${acct.mr}` : 'Overview'),
      h('div', { class: 'cards' }, card('Credits', fmtN(acct.credits)), card('Platinum', fmtN(acct.platinum)), card('Endo', fmtN(acct.endo)), card('Foundry jobs', acct.foundry.length), card('Items', fmtN(acct.inv.size))),
      h('h3', {}, 'What you are close to completing'),
      close.length ? h('pre', {}, close.map((c) => describe(c, false)).join('\n')) : h('p', { class: 'dim' }, 'Nothing partway yet. Get a Prime part or add a goal.'),
      h('div', { class: 'row' }, h('button', { class: 'b', onclick: () => { chat.push({ me: true, t: 'What am I close to completing? How do I get them?' }); chat.push({ t: answer(ctx(), 'what am i close to completing? how do i get them') }); go('Ordis'); } }, 'Show me how to get them')));
  },
  Ordis() {
    const input = h('input', { type: 'text', placeholder: 'Ask Ordis… e.g. "what am I close to completing?"', style: 'flex:1;min-width:300px' });
    const send = () => { const q = input.value.trim(); if (!q) return; chat.push({ me: true, t: q }, { t: answer(ctx(), q) }); go('Ordis'); setTimeout(() => $('#q') && $('#q').focus(), 0); };
    input.id = 'q'; input.addEventListener('keydown', (e) => { if (e.key === 'Enter') send(); });
    return h('div', {}, h('h2', {}, 'Ordis'),
      h('div', { class: 'chat' }, (chat.length ? chat : [{ t: HELP }]).map((m) => h('div', { class: `msg${m.me ? ' me' : ''}` }, h('pre', {}, m.t)))),
      h('div', { class: 'row' }, input, h('button', { class: 'b', onclick: send }, 'Ask')));
  },
  Goals() {
    const { acct, goals } = ctx();
    const kind = h('select', {}, ...[['build', 'Build an item'], ['collect', 'Collect a resource'], ['mastery', 'Reach mastery rank']].map(([v, l]) => h('option', { value: v }, l)));
    const q = h('input', { type: 'text', placeholder: 'Item or resource name', style: 'min-width:240px' });
    const n = h('input', { type: 'number', value: '1', min: '1', style: 'width:90px' });
    const msg = h('span', { class: 'dim' });
    const add = () => {
      let g;
      if (kind.value === 'mastery') g = { kind: 'mastery', count: +n.value || 1, name: `Mastery rank ${+n.value || 1}` };
      else {
        const it = kind.value === 'build' ? cat.findItem(q.value) : null;
        const hit = it ? { unique: it.unique, name: it.name } : (cat.searchNames(q.value, 1)[0] ? { unique: cat.searchNames(q.value, 1)[0][0], name: cat.display(cat.searchNames(q.value, 1)[0][0]) } : null);
        if (!hit) { msg.textContent = 'Ordis could not find that.'; return; }
        g = { kind: kind.value, target: hit.unique, name: hit.name, count: kind.value === 'collect' ? +n.value || 1 : 1 };
      }
      bg.saveGoals([...goals, g]); go('Goals');
    };
    return h('div', {}, h('h2', {}, 'Goals'),
      h('div', { class: 'row' }, kind, q, n, h('button', { class: 'b', onclick: add }, 'Add'), msg),
      goals.length ? table(['Goal', 'Progress', 'Status', ''], goals.map((g, i) => {
        const s = goalStatus(cat, acct, g);
        return [g.name || g.target, bar(s.fraction), h('span', { class: s.done ? 'good' : '' }, s.done ? 'Done' : s.summary),
          h('button', { class: 'b alt', onclick: () => { bg.saveGoals(goals.filter((_, j) => j !== i)); go('Goals'); } }, 'Remove')];
      })) : h('p', { class: 'dim' }, 'No goals yet. Ordis also tracks every Prime you hold a part of automatically.'));
  },
  Inventory() {
    const { acct } = ctx();
    const f = h('input', { type: 'text', placeholder: 'Filter…', value: filter, oninput: (e) => { filter = e.target.value; draw(); } });
    const q = filter.toLowerCase();
    const rows = [...acct.inv.entries()].filter(([, e]) => e.count > 0).map(([u, e]) => [cat.display(u), e.count, u]).filter((r) => !q || r[0].toLowerCase().includes(q)).sort((a, b) => a[0].localeCompare(b[0]));
    return h('div', {}, h('h2', {}, `Inventory (${fmtN(rows.length)})`), h('div', { class: 'row' }, f), table(['Item', 'Count'], rows.slice(0, 400).map((r) => [r[0], fmtN(r[1])])), rows.length > 400 ? h('p', { class: 'dim' }, 'Showing the first 400. Use the filter.') : null);
  },
  Relics() {
    const { acct, goals } = ctx();
    const rows = relicRows(cat, acct, neededParts(cat, acct, goals).needed);
    return h('div', {}, h('h2', {}, 'Relics'), h('p', { class: 'dim' }, 'Relics holding a part one of your goals still needs come first.'),
      table(['Relic', 'Refinement', 'Owned', 'Useful rewards'], rows.map((r) => [r.name, r.refinement, r.count, r.rewards.filter((x) => x.useful).map((x) => x.name).join(', ') || '—'])));
  },
  Mastery() {
    const { acct } = ctx();
    const rows = masteryRows(cat, acct);
    const by = {}; for (const r of rows) { (by[r.category] ||= { total: 0, done: 0 }); by[r.category].total++; if (r.state === 'mastered') by[r.category].done++; }
    const done = rows.filter((r) => r.state === 'mastered').length;
    const todo = rows.filter((r) => r.state !== 'mastered' && (!r.mrReq || r.mrReq <= acct.mr));
    return h('div', {}, h('h2', {}, `Mastered ${done} of ${rows.length}`),
      table(['Category', 'Done', ''], Object.entries(by).sort().map(([k, v]) => [k, `${v.done} / ${v.total}`, bar(v.done / v.total)])),
      h('h3', {}, 'Available to master'), table(['Item', 'Category', 'State'], todo.slice(0, 300).map((r) => [r.name, r.category, r.state === 'owned' ? 'Owned, needs XP' : 'Not owned'])));
  },
  Foundry() {
    const { acct } = ctx();
    const rows = foundryRows(cat, acct);
    const can = buildableNow(cat, acct);
    return h('div', {}, h('h2', {}, 'Foundry'),
      rows.length ? table(['Item', 'Status'], rows.map((r) => [r.name, r.remainingMs <= 0 ? h('span', { class: 'good' }, 'Ready to collect') : `${fmtDuration(r.remainingMs)} left`])) : h('p', { class: 'dim' }, 'Nothing building.'),
      h('h3', {}, `Blueprints you could build now (${can.length})`),
      h('pre', {}, can.slice(0, 60).map((c) => `• ${cat.display(c)}`).join('\n') || 'None.'));
  },
  Settings() {
    const st = bg.state, s = st.status;
    const name = h('input', { type: 'text', value: st.settings.name || '', placeholder: 'Your name' });
    return h('div', {}, h('h2', {}, 'Settings and diagnostics'),
      h('div', { class: 'row' }, name, h('button', { class: 'b', onclick: () => bg.saveSettings({ ...st.settings, name: name.value }) }, 'Save name')),
      h('h3', {}, 'Game connection'),
      table(['', ''], [['Ordis Nexus version', st.version], ['Warframe running', s.gameRunning ? 'yes' : 'no'], ['GEP features registered', s.featuresSet ? 'yes' : 'no'], ['Last error', s.lastError || '—'],
        ['Messages received', s.messages], ['With inventory', s.withInventory], ['Partial (ignored)', s.partial], ['Last partial keys', s.lastPartialKeys.join(', ') || '—'],
        ['Inventory stored', st.acct ? `${fmtN(st.acct.inv.size)} items, ${ago(st.updatedAt)}` : 'none'], ['Catalog', cat ? `${cat.primes.length} primes loaded` : st.catalogError || 'loading']]),
      h('div', { class: 'row' }, h('button', { class: 'b alt', onclick: () => bg.refresh() }, 'Ask the game for info now'),
        h('button', { class: 'b alt', onclick: () => { bg.forgetInventory(); } }, 'Forget stored inventory')),
      h('p', { class: 'dim' }, 'Updates: Ordis Nexus updates through Overwolf. There is no separate updater.'),
      h('h3', {}, 'Recent messages'), h('pre', { class: 'dim' }, s.recent.map((m) => `${new Date(m.at).toLocaleTimeString()}  ${m.kind}  ${m.summary}`).join('\n') || 'None yet.'));
  },
};

function go(t) { tab = t; draw(); }
function draw() {
  if (!bg) return;
  $('#tabs').replaceChildren(...TABS.map((t) => h('button', { class: t === tab ? 'on' : '', onclick: () => go(t) }, t)));
  pill();
  const main = $('#view');
  const keep = document.activeElement && document.activeElement.id === 'q';
  const ready = cat && (bg.state.acct || tab === 'Settings' || tab === 'Ordis' || tab === 'Goals');
  if (!ready || (tab !== 'Settings' && tab !== 'Ordis' && !bg.state.acct)) { main.replaceChildren(tab === 'Settings' ? views.Settings() : needData(tab)); return; }
  if (!bg.state.acct && (tab === 'Goals')) { main.replaceChildren(needData(tab)); return; }
  main.replaceChildren(views[tab]());
  if (keep && $('#q')) $('#q').focus();
}

overwolf.windows.getMainWindow((w) => {
  bg = w.ordis;
  bg.subscribe(() => { if (document.activeElement && ['INPUT', 'SELECT'].includes(document.activeElement.tagName) && tab !== 'Settings') { pill(); return; } draw(); });
  bg.catalog.then((c) => { cat = c; draw(); }, () => draw());
  draw();
  setInterval(draw, 30000);
});
