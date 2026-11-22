"use strict";
(function () {
const __m = {};
__m["core/util.js"] = (function () {
// Small helpers shared by the engine. Pure functions only (no DOM, no Overwolf).

function swapSuffix(unique, from, to) {
  const n = unique.length;
  if (n >= from.length && unique.slice(n - from.length).toLowerCase() === from.toLowerCase()) {
    const tail = unique.slice(n - from.length);
    const t = tail === tail.toLowerCase() ? to.toLowerCase() : to; // keep lowercase paths lowercase
    return unique.slice(0, n - from.length) + t;
  }
  return unique;
}

const blueprintForm = (u) => swapSuffix(u, 'Component', 'Blueprint');
const componentForm = (u) => swapSuffix(u, 'Blueprint', 'Component');

/** `/Lotus/Types/Items/MiscItems/OrokinCatalyst` -> `Orokin Catalyst` */
function prettyName(path) {
  const last = path.split('/').pop() || path;
  return prettyRaw(last);
}

function prettyRaw(last) {
  let out = '';
  for (let i = 0; i < last.length; i++) {
    const c = last[i];
    if (i > 0 && c !== c.toLowerCase() && c === c.toUpperCase()) {
      const prev = last[i - 1];
      const next = last[i + 1];
      const nextLower = next !== undefined && next === next.toLowerCase() && next !== next.toUpperCase();
      const prevLower = prev === prev.toLowerCase() && prev !== prev.toUpperCase();
      const prevUpper = prev === prev.toUpperCase() && prev !== prev.toLowerCase();
      const prevDigit = prev >= '0' && prev <= '9';
      if (prevLower || prevDigit || (prevUpper && nextLower)) out += ' ';
    }
    out += c;
  }
  return out;
}

const fmtN = (n) => Math.round(n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');

function fmtDuration(ms) {
  if (ms <= 0) return 'now';
  let s = Math.floor(ms / 1000);
  const d = Math.floor(s / 86400); s %= 86400;
  const h = Math.floor(s / 3600); s %= 3600;
  const m = Math.floor(s / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

return { swapSuffix, blueprintForm, componentForm, prettyName, prettyRaw, fmtN, fmtDuration };
})();
__m["core/account.js"] = (function () {
// A player's whole state, parsed from the Warframe inventory JSON that Overwolf's
// Game Events Provider delivers as `match_info.inventory`.

// Arrays that mention ItemType but are history/progress, not things you own.
const IGNORED = new Set(['XPInfo', 'LoreFragmentScans', 'PersonalTechProjects', 'PendingRecipes', 'Boosters']);

const OWNED_ARRAYS = ['Suits', 'LongGuns', 'Pistols', 'Melee', 'SpaceSuits', 'SpaceGuns', 'SpaceMelee',
  'Sentinels', 'KubrowPets', 'MechSuits', 'SentinelWeapons', 'Horses'];

const CATEGORY = {
  Suits: 'Warframes', LongGuns: 'Primary weapons', Pistols: 'Secondary weapons', Melee: 'Melee weapons',
  RawUpgrades: 'Mods', Upgrades: 'Mods', MiscItems: 'Resources & misc', Recipes: 'Blueprints',
  SpaceSuits: 'Archwings', SpaceGuns: 'Archwing weapons', SpaceMelee: 'Archwing weapons',
  Sentinels: 'Sentinels', SentinelWeapons: 'Sentinel weapons', KubrowPets: 'Companions',
  FlavourItems: 'Cosmetics', WeaponSkins: 'Skins', MechSuits: 'Necramechs', OperatorAmps: 'Amps',
  CrewShipSalvagedWeapons: 'Railjack weapons', CrewShipWeapons: 'Railjack weapons',
};

const { prettyRaw } = __m["core/util.js"];

function categoryName(key) {
  return CATEGORY[key] || prettyRaw(key);
}

const num = (v) => (typeof v === 'number' && isFinite(v) ? Math.max(0, Math.floor(v)) : 0);

/** Mongo-style dates: {"$date":{"$numberLong":"170..."}} or {"$date":170...} */
function jsonMs(v) {
  if (v == null) return null;
  const d = v.$date !== undefined ? v.$date : v;
  if (typeof d === 'number') return d;
  if (d && d.$numberLong !== undefined) {
    const n = Number(d.$numberLong);
    return isFinite(n) ? n : null;
  }
  return null;
}

/** A whole-account snapshot has the core fields; a delta must never replace one. */
function looksComplete(root) {
  return !!root && typeof root === 'object' && ['PlayerLevel', 'RegularCredits', 'MiscItems'].every((k) => root[k] !== undefined);
}

function parseAccount(root) {
  const inv = new Map(); // path -> {count, category}
  const lower = new Map(); // lowercase path -> count
  for (const [key, arr] of Object.entries(root || {})) {
    if (IGNORED.has(key) || !Array.isArray(arr)) continue;
    const category = categoryName(key);
    for (const it of arr) {
      if (!it || typeof it.ItemType !== 'string') continue;
      const n = it.ItemCount === undefined ? 1 : num(it.ItemCount);
      const e = inv.get(it.ItemType) || { count: 0, category };
      e.count += n;
      inv.set(it.ItemType, e);
    }
  }
  for (const [p, e] of inv) lower.set(p.toLowerCase(), (lower.get(p.toLowerCase()) || 0) + e.count);

  const xp = new Map();
  for (const x of (root && root.XPInfo) || []) {
    if (x && typeof x.ItemType === 'string' && typeof x.XP === 'number') {
      xp.set(x.ItemType, Math.max(xp.get(x.ItemType) || 0, x.XP));
    }
  }
  const owned = new Set();
  for (const key of OWNED_ARRAYS) {
    for (const x of (root && root[key]) || []) if (x && typeof x.ItemType === 'string') owned.add(x.ItemType);
  }
  const foundry = [];
  for (const j of (root && root.PendingRecipes) || []) {
    const ms = jsonMs(j && j.CompletionDate);
    if (j && typeof j.ItemType === 'string' && ms !== null) foundry.push({ recipe: j.ItemType, doneMs: ms });
  }
  foundry.sort((a, b) => a.doneMs - b.doneMs);

  return {
    inv, lower, xp, owned,
    ownedLower: new Set([...owned].map((s) => s.toLowerCase())),
    mr: num(root && root.PlayerLevel),
    credits: num(root && root.RegularCredits),
    platinum: num(root && root.PremiumCredits) + num(root && root.PremiumCreditsFree),
    endo: num(root && root.FusionPoints),
    foundry,
  };
}

const count = (acct, unique) => acct.lower.get(unique.toLowerCase()) || 0;
const isEmpty = (acct) => acct.inv.size === 0;

/** Relic items: [path, copies] */
function relicsOf(acct) {
  const out = [];
  for (const [k, e] of acct.inv) if (k.includes('/Projections/') && k.includes('VoidProjection')) out.push([k, e.count]);
  return out;
}

/** Items gained between two accounts: [[path, delta]] (positive only). */
function gained(before, after) {
  const out = [];
  for (const [p, e] of after.inv) {
    const d = e.count - (before.inv.get(p)?.count || 0);
    if (d > 0) out.push([p, d]);
  }
  return out.sort((a, b) => b[1] - a[1]);
}

return { categoryName, jsonMs, looksComplete, parseAccount, count, isEmpty, relicsOf, gained };
})();
__m["core/gep.js"] = (function () {
// Pulls the account inventory out of Overwolf Game Events Provider messages.
// The same JSON document shows up in a few shapes depending on the call:
//   onInfoUpdates2: { feature, category, key: 'inventory', value: '<json string>' }
//   getInfo():      { info: { match_info: { inventory: '<json string>' } } }
const { looksComplete } = __m["core/account.js"];

function findInventory(v, depth = 0) {
  if (v === null || typeof v !== 'object' || depth > 6) return undefined;
  if (typeof v.key === 'string' && v.key === 'inventory' && v.value !== undefined) return v.value;
  if (v.inventory !== undefined) return v.inventory;
  for (const k of Object.keys(v)) {
    const r = findInventory(v[k], depth + 1);
    if (r !== undefined) return r;
  }
  return undefined;
}

/** -> { hasInventory, complete, root, keys } ; never throws. */
function extractInventory(msg) {
  const raw = findInventory(msg);
  if (raw === undefined) return { hasInventory: false, complete: false, root: null, keys: [] };
  let root = raw;
  if (typeof raw === 'string') {
    try { root = JSON.parse(raw); } catch { return { hasInventory: true, complete: false, root: null, keys: [] }; }
  }
  const keys = root && typeof root === 'object' ? Object.keys(root).slice(0, 40) : [];
  return { hasInventory: true, complete: looksComplete(root), root, keys };
}

return { findInventory, extractInventory };
})();
__m["core/catalog.js"] = (function () {
// The item database: names, recipes, relics, drop tables and resource hints.
// Built from data/catalog.json.gz (generated by scripts/gen_catalog.py).

const { blueprintForm, componentForm, prettyName } = __m["core/util.js"];

const REFINEMENTS = [['Bronze', 'Intact'], ['Silver', 'Exceptional'], ['Gold', 'Flawless'], ['Platinum', 'Radiant']];

class Catalog {
  constructor(raw) {
    this.names = raw.names || {};
    this.icons = raw.icons || {};
    this.primes = (raw.primes || []).map((p) => ({
      unique: p.u, name: p.n, masteryXp: p.x,
      parts: (p.p || []).map(([unique, name, need]) => ({ unique, name, need })),
    }));
    this.recipes = new Map();
    this.byBlueprint = new Map();
    for (const [k, r] of Object.entries(raw.recipes || {})) {
      this.byBlueprint.set(r.b.toLowerCase(), k);
      this.recipes.set(k, {
        result: r.r, blueprint: r.b, credits: r.c || 0, secs: r.t || 0, num: Math.max(1, r.n || 1),
        ingredients: r.i || [],
      });
    }
    this.items = (raw.items || []).map((i) => ({ unique: i.u, name: i.n, category: i.c || '', mr: i.m || 0, masteryXp: i.x || 0 }));
    this.itemIndex = new Map(this.items.map((it, i) => [it.unique.toLowerCase(), i]));
    this.relics = new Map();
    this.rewardIndex = new Map();
    for (const [k, r] of Object.entries(raw.relics || {})) {
      const rewards = (r.rw || []).map(([unique, rarity, count]) => ({ unique, rarity, count }));
      for (const rw of rewards) {
        const key = rw.unique.toLowerCase();
        if (!this.rewardIndex.has(key)) this.rewardIndex.set(key, []);
        this.rewardIndex.get(key).push([k, rw.rarity]);
      }
      this.relics.set(k, { name: r.n, era: (r.n || '').split(/\s+/)[0] || '', rewards });
    }
    this.res = raw.res || {};
    this.slugs = raw.slugs || {};
    this.drops = new Map(Object.entries(raw.drops || {}).map(([k, v]) => [k, v.map(([place, rarity, chance]) => ({ place, rarity: (rarity || '?')[0], chance }))]));
    this.nameList = Object.entries(this.names).map(([u, n]) => [n.toLowerCase(), u]);
  }

  name(unique) { return this.names[unique.toLowerCase()]; }

  /** Display name with a readable fallback derived from the path. */
  display(unique) {
    const n = this.name(unique);
    if (n) return n;
    const b = this.name(blueprintForm(unique));
    if (b) return b.replace(/ Blueprint$/, '');
    return prettyName(unique);
  }

  icon(unique) {
    const l = unique.toLowerCase();
    return this.icons[l] || this.icons[componentForm(l)] || this.icons[blueprintForm(l)] || null;
  }

  recipe(result) { return this.recipes.get(result.toLowerCase()) || null; }

  resultOfBlueprint(bp) {
    const key = this.byBlueprint.get(bp.toLowerCase());
    return key ? this.recipes.get(key).result : null;
  }

  item(unique) {
    const i = this.itemIndex.get(unique.toLowerCase());
    return i === undefined ? null : this.items[i];
  }

  /** Best match for a typed name: exact, then primes, then shortest containing name. */
  findItem(query) {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    const exact = this.items.find((i) => i.name.toLowerCase() === q);
    if (exact) return exact;
    const hits = this.items.filter((i) => i.name.toLowerCase().includes(q));
    hits.sort((a, b) => (a.name.includes('Prime') ? 0 : 1) - (b.name.includes('Prime') ? 0 : 1) || a.name.length - b.name.length);
    return hits[0] || null;
  }

  /** The masterable item whose name appears in `text` (longest name wins). */
  mentionedItem(text) {
    const t = text.toLowerCase();
    let best = null;
    for (const i of this.items) {
      if (i.name.length >= 3 && t.includes(i.name.toLowerCase()) && (!best || i.name.length > best.name.length)) best = i;
    }
    return best;
  }

  /** Any named thing: [[lowercase unique, name]] */
  searchNames(query, limit = 20) {
    const q = query.trim().toLowerCase();
    if (q.length < 2) return [];
    const hits = this.nameList.filter(([n]) => n.includes(q));
    hits.sort((a, b) => (a[0] === q ? 0 : 1) - (b[0] === q ? 0 : 1) || a[0].length - b[0].length);
    return hits.slice(0, limit).map(([, u]) => [u, this.names[u]]);
  }

  /** Names inside free text, longest first, skipping names swallowed by a longer match. */
  mentioned(text, limit = 3) {
    const t = text.toLowerCase();
    const hits = this.nameList.filter(([n]) => n.length >= 4 && t.includes(n));
    hits.sort((a, b) => b[0].length - a[0].length || (a[0] < b[0] ? -1 : 1));
    const out = [];
    for (const [, u] of hits) {
      const name = this.names[u];
      if (out.some(([, n]) => n.toLowerCase().includes(name.toLowerCase()))) continue;
      out.push([u, name]);
      if (out.length >= limit) break;
    }
    return out;
  }

  slug(unique) {
    const l = unique.toLowerCase();
    return this.slugs[l] || this.slugs[blueprintForm(l)] || this.slugs[componentForm(l)] || null;
  }

  resourceHint(unique) { return this.res[unique.toLowerCase()] || null; }

  /** Non-relic drop sources by display name, best chance first. */
  dropsFor(displayName) {
    return [...(this.drops.get(displayName) || [])].sort((a, b) => b.chance - a.chance);
  }

  /** [relic, refinement] for an inventory relic path like ...T4VoidProjectionXPrimeASilver */
  relic(inventoryUnique) {
    const l = inventoryUnique.toLowerCase();
    for (const [suffix, label] of REFINEMENTS) {
      const s = suffix.toLowerCase();
      if (l.endsWith(s)) {
        const r = this.relics.get(l.slice(0, l.length - s.length));
        if (r) return [r, label];
      }
    }
    return null;
  }

  /** Relics that can drop a part: [[relic, rarity]] */
  relicsWith(rewardUnique) {
    const l = rewardUnique.toLowerCase();
    const out = [];
    for (const key of [l, blueprintForm(l), componentForm(l)]) {
      for (const [base, rarity] of this.rewardIndex.get(key) || []) {
        const r = this.relics.get(base);
        if (r && !out.some(([x]) => x === r)) out.push([r, rarity]);
      }
    }
    return out;
  }
}

/** Load the bundled gzip catalog in a browser/Overwolf window. */
async function loadCatalogFromUrl(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`catalog fetch failed: ${res.status}`);
  const stream = res.body.pipeThrough(new DecompressionStream('gzip'));
  const text = await new Response(stream).text();
  return new Catalog(JSON.parse(text));
}

return { REFINEMENTS, Catalog, loadCatalogFromUrl };
})();
__m["background.js"] = (function () {
// Background window: always running. Registers for Warframe game events, keeps the
// latest complete inventory, and shares state with the main window.
const { parseAccount } = __m["core/account.js"];
const { extractInventory } = __m["core/gep.js"];
const { loadCatalogFromUrl } = __m["core/catalog.js"];

const startupErrors = [];
window.addEventListener('error', (e) => startupErrors.push(String(e.message || e)));
window.addEventListener('unhandledrejection', (e) => startupErrors.push(String((e.reason && e.reason.message) || e.reason)));
const safe = (what, fn) => { try { fn(); } catch (e) { startupErrors.push(`${what}: ${e && e.message ? e.message : e}`); } };

const GAME_ID = 8954; // Warframe (verify in Overwolf's game list if nothing arrives)
const FEATURES = ['match_info', 'game_info'];
const KEY = { inv: 'ordis.inventory', meta: 'ordis.meta', goals: 'ordis.goals', settings: 'ordis.settings' };
const store = {
  get(k, d) { try { const v = localStorage.getItem(k); return v === null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); return true; } catch { return false; } },
};

const state = {
  version: '0.4.5',
  acct: null,
  updatedAt: 0,
  goals: store.get(KEY.goals, []),
  settings: store.get(KEY.settings, { name: '' }),
  status: { gameRunning: false, featuresSet: false, game: null, lastError: '', messages: 0, withInventory: 0, partial: 0, lastPartialKeys: [], lastKeys: [], recent: [] },
  catalogError: '',
  startupErrors,
};
const listeners = new Set();
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* a closed window */ } });

const catalog = loadCatalogFromUrl('data/catalog.json.gz').catch((e) => { state.catalogError = String(e); notify(); throw e; });

function adopt(root, ts) {
  state.acct = parseAccount(root);
  state.updatedAt = ts;
}

// restore the last complete inventory from the previous session
(function restore() {
  const root = store.get(KEY.inv, null);
  if (root) { try { adopt(root, store.get(KEY.meta, {}).updatedAt || 0); } catch { /* ignore */ } }
})();

function log(kind, data) {
  const st = state.status;
  st.messages++;
  st.recent.unshift({ at: Date.now(), kind, summary: JSON.stringify(data).slice(0, 300) });
  st.recent.length = Math.min(st.recent.length, 25);
}

function handle(kind, msg) {
  log(kind, msg);
  const r = extractInventory(msg);
  const st = state.status;
  if (r.hasInventory) {
    st.withInventory++;
    st.lastKeys = r.keys;
    if (r.complete) {
      adopt(r.root, Date.now());
      store.set(KEY.inv, r.root);
      store.set(KEY.meta, { updatedAt: state.updatedAt });
    } else {
      st.partial++;
      st.lastPartialKeys = r.keys;
    }
  }
  notify();
}

function setFeatures(attempt = 0) {
  overwolf.games.events.setRequiredFeatures(FEATURES, (res) => {
    if (res && res.success) {
      state.status.featuresSet = true; state.status.lastError = '';
      overwolf.games.events.getInfo((info) => handle('getInfo', info));
    } else {
      state.status.featuresSet = false;
      state.status.lastError = `setRequiredFeatures: ${(res && (res.error || res.reason)) || 'failed'}`;
      setTimeout(() => setFeatures(attempt + 1), 5000);
    }
    notify();
  });
}

function onGame(info) {
  if (info) state.status.game = { id: info.id, classId: info.classId, title: info.title, isRunning: !!info.isRunning };
  const running = !!(info && info.isRunning && (Math.floor((info.id || 0) / 10) === GAME_ID || /warframe/i.test(info.title || '')));
  if (running !== state.status.gameRunning) {
    state.status.gameRunning = running;
    if (running) setFeatures();
    notify();
  }
}

safe('onInfoUpdates2', () => overwolf.games.events.onInfoUpdates2.addListener((m) => handle('info', m)));
safe('onNewEvents', () => overwolf.games.events.onNewEvents.addListener((m) => { log('event', m); notify(); }));
safe('onGameInfoUpdated', () => overwolf.games.onGameInfoUpdated.addListener((e) => onGame(e && e.gameInfo)));
safe('getRunningGameInfo', () => overwolf.games.getRunningGameInfo((info) => onGame(info)));
setInterval(() => safe('poll', () => overwolf.games.getRunningGameInfo((i) => { onGame(i); notify(); })), 5000);
setInterval(() => { if (state.status.gameRunning) overwolf.games.events.getInfo((i) => handle('getInfo', i)); }, 60000);

// window handling
function showMain() { overwolf.windows.obtainDeclaredWindow('main', (r) => { if (r.success) overwolf.windows.restore(r.window.id, () => {}); }); }
safe('onAppLaunchTriggered', () => overwolf.extensions.onAppLaunchTriggered.addListener((e) => { if (!e || e.origin !== 'gamelaunchevent') showMain(); }));
safe('hotkeys', () => overwolf.settings.hotkeys.onPressed.addListener((e) => {
  if (e.name !== 'toggle_main') return;
  overwolf.windows.obtainDeclaredWindow('main', (r) => {
    if (!r.success) return;
    overwolf.windows.getWindowState(r.window.id, (s) => {
      const st = s.window_state_ex || s.window_state;
      if (st === 'normal' || st === 'maximized') overwolf.windows.minimize(r.window.id, () => {}); else overwolf.windows.restore(r.window.id, () => {});
    });
  });
}));

// API for the main window (overwolf.windows.getMainWindow() returns this window)
window.ordis = {
  state, catalog,
  subscribe(f) { listeners.add(f); return () => listeners.delete(f); },
  saveGoals(g) { state.goals = g; store.set(KEY.goals, g); notify(); },
  saveSettings(s) { state.settings = s; store.set(KEY.settings, s); notify(); },
  forgetInventory() { state.acct = null; state.updatedAt = 0; try { localStorage.removeItem(KEY.inv); } catch { /* ignore */ } notify(); },
  refresh() { overwolf.games.events.getInfo((i) => handle('getInfo', i)); },
};
safe('setFeatures', () => setFeatures());
// The launch event can fire before this script runs, so open the window ourselves
// unless Overwolf started us silently because the game launched.
if (!/gamelaunchevent/.test(window.location.href)) setTimeout(showMain, 300);

return {  };
})();
})();
