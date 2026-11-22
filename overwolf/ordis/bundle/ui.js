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
__m["core/recipes.js"] = (function () {
// What building an item needs, given what the account owns.
const { count } = __m["core/account.js"];

const MAX_DEPTH = 4;

function resolve(cat, acct, target, qty = 1) {
  const rec = cat.recipe(target);
  if (!rec) return null;
  const out = { target: rec.result, rootBlueprint: count(acct, rec.blueprint), parts: [], resources: [], credits: rec.credits * qty, secs: rec.secs * qty };
  const res = new Map();
  for (const [ing, n] of rec.ingredients) expand(cat, acct, ing, n * qty, 1, out, res);
  out.resources = [...res.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).map(([unique, need]) => ({ unique, need, have: count(acct, unique) }));
  out.parts.sort((a, b) => (a.unique < b.unique ? -1 : 1));
  return out;
}

function expand(cat, acct, ing, need, depth, out, res) {
  const sub = depth <= MAX_DEPTH ? cat.recipe(ing) : null;
  if (!sub) {
    res.set(ing, (res.get(ing) || 0) + need);
    return;
  }
  const built = count(acct, ing);
  const blueprints = count(acct, sub.blueprint);
  const fromBuilt = Math.min(need, built);
  const toCraft = need - fromBuilt;
  const toObtain = Math.max(0, toCraft - blueprints);
  const existing = out.parts.find((p) => p.unique === ing);
  if (existing) {
    existing.need += need;
    existing.toObtain += toObtain;
  } else {
    out.parts.push({ unique: ing, need, built, blueprints, toObtain });
  }
  if (toCraft > 0) {
    out.credits += sub.credits * toCraft;
    out.secs += sub.secs * toCraft;
    for (const [i2, n2] of sub.ingredients) expand(cat, acct, i2, n2 * toCraft, depth + 1, out, res);
  }
}

const partsMissing = (r) => r.parts.filter((p) => p.toObtain > 0);
const resMissing = (r) => r.resources.filter((x) => x.need > x.have);
const missing = (x) => Math.max(0, x.need - x.have);
const buildable = (r, acct) => partsMissing(r).length === 0 && resMissing(r).length === 0 && acct.credits >= r.credits;

/** 0..1, averaged over parts and resource totals. */
function fraction(r) {
  let total = 0, n = 0;
  for (const p of r.parts) { total += Math.max(0, p.need - p.toObtain) / Math.max(1, p.need); n++; }
  for (const x of r.resources) { total += Math.min(x.have, x.need) / Math.max(1, x.need); n++; }
  return n === 0 ? 1 : total / n;
}

return { resolve, partsMissing, resMissing, missing, buildable, fraction };
})();
__m["core/goals.js"] = (function () {
// Goals: build an item, collect N of something, or reach a mastery rank.
const { count } = __m["core/account.js"];
const { resolve, partsMissing, resMissing, missing, buildable, fraction } = __m["core/recipes.js"];

function goalStatus(cat, acct, goal) {
  const empty = { done: false, fraction: 0, summary: '', missingParts: [], missingResources: [], resolution: null };
  if (goal.kind === 'mastery') {
    const target = Math.max(1, goal.count || 1);
    return { ...empty, done: acct.mr >= target, fraction: Math.min(1, acct.mr / target), summary: `Mastery rank ${acct.mr} / ${target}` };
  }
  if (goal.kind === 'collect') {
    const have = count(acct, goal.target);
    const need = Math.max(1, goal.count || 1);
    const miss = Math.max(0, need - have);
    return { ...empty, done: miss === 0, fraction: Math.min(1, have / need), summary: `${have} / ${need}`, missingResources: miss > 0 ? [[goal.target, miss]] : [] };
  }
  // build
  const owned = acct.ownedLower.has((goal.target || '').toLowerCase());
  const res = resolve(cat, acct, goal.target, 1);
  if (!res) return { ...empty, done: owned, fraction: owned ? 1 : 0, summary: owned ? 'Owned' : 'No recipe data for this item' };
  if (owned) return { ...empty, done: true, fraction: 1, summary: 'Built', resolution: res };
  const missingParts = partsMissing(res).map((p) => [p.unique, p.toObtain]);
  const missingResources = resMissing(res).map((x) => [x.unique, missing(x)]);
  const summary = buildable(res, acct) ? 'Ready to build'
    : missingParts.length ? `${missingParts.reduce((a, p) => a + p[1], 0)} part(s) to obtain`
      : `${missingResources.length} resource type(s) short`;
  return { ...empty, fraction: fraction(res), summary, missingParts, missingResources, resolution: res };
}

return { goalStatus };
})();
__m["core/close.js"] = (function () {
// "What am I close to completing?" Ranks things you could finish soon and says
// exactly what is missing and where to get it, using only the stored catalog.
const { relicsOf } = __m["core/account.js"];
const { resolve, partsMissing, resMissing, missing, buildable, fraction } = __m["core/recipes.js"];
const { blueprintForm, fmtN } = __m["core/util.js"];

const rarityWord = (r) => ({ c: 'common', u: 'uncommon', r: 'rare' }[(r || '')[0]?.toLowerCase()] || r);

/** How to get a Prime part: relics that drop it, marking ones you already hold. */
function howPart(cat, acct, partUnique) {
  const relics = cat.relicsWith(blueprintForm(partUnique));
  if (relics.length === 0) return 'no relic source on file (check the Prime Vault or the item\'s wiki page)';
  const held = new Set();
  for (const [k] of relicsOf(acct)) { const h = cat.relic(k); if (h) held.add(h[0].name); }
  const sorted = [...relics].sort((a, b) => Number(!held.has(a[0].name)) - Number(!held.has(b[0].name)));
  const list = sorted.slice(0, 4).map(([r, rar]) => `${r.name} (${rarityWord(rar)}${held.has(r.name) ? ', you own it' : ''})`);
  const more = relics.length - 4;
  return `relics: ${list.join(', ')}${more > 0 ? ` and ${more} more` : ''}`;
}

/** How to get a resource or other item: our hint text, else the best drop tables. */
function howItem(cat, unique) {
  const hint = cat.resourceHint(unique);
  if (hint) return hint;
  const drops = cat.dropsFor(cat.display(unique));
  if (drops.length === 0) return 'no source on file';
  return `drops from ${drops.slice(0, 3).map((d) => `${d.place} (${d.chance.toFixed(1)}%)`).join('; ')}`;
}

function buildClose(cat, acct, unique, name, isGoal) {
  const res = resolve(cat, acct, unique, 1);
  if (!res) return null;
  return {
    unique, name, isGoal, fraction: fraction(res),
    parts: partsMissing(res).map((p) => ({ name: cat.display(blueprintForm(p.unique)), count: p.toObtain, how: howPart(cat, acct, p.unique) })),
    resources: resMissing(res).map((r) => ({ name: cat.display(r.unique), need: r.need, have: r.have, how: howItem(cat, r.unique) })),
    creditsShort: Math.max(0, res.credits - acct.credits),
    buildable: buildable(res, acct),
  };
}

/** Smaller is closer. */
const effort = (c) => c.parts.reduce((a, p) => a + p.count, 0) * 100 + c.resources.length * 8 + (c.creditsShort > 0 ? 5 : 0);

/** Unfinished Build goals plus every Prime you hold at least one part of, closest first. */
function closeToCompleting(cat, acct, goals, limit = 5) {
  const owned = (u) => acct.ownedLower.has(u.toLowerCase());
  const out = [];
  const seen = new Set();
  for (const g of goals.filter((g) => g.kind === 'build' && g.target)) {
    if (owned(g.target) || seen.has(g.target.toLowerCase())) continue;
    seen.add(g.target.toLowerCase());
    const c = buildClose(cat, acct, g.target, g.name, true);
    if (c) out.push(c);
  }
  for (const p of cat.primes) {
    if (owned(p.unique) || seen.has(p.unique.toLowerCase())) continue;
    seen.add(p.unique.toLowerCase());
    const c = buildClose(cat, acct, p.unique, p.name, false);
    if (c && c.fraction > 0) out.push(c);
  }
  out.sort((a, b) => effort(a) - effort(b) || b.fraction - a.fraction || a.name.localeCompare(b.name));
  return out.slice(0, limit);
}

function describe(c, detailed) {
  const tag = c.isGoal ? ' (a goal)' : '';
  if (c.buildable) return `• ${c.name}${tag}: you have everything. Start the build in the Foundry.`;
  const head = `• ${c.name}${tag}: ${Math.round(c.fraction * 100)}% there`;
  if (!detailed) {
    const bits = [];
    const parts = c.parts.reduce((a, p) => a + p.count, 0);
    if (parts) bits.push(`${parts} part(s) to find`);
    if (c.resources.length) bits.push(`${c.resources.length} resource(s) short`);
    if (c.creditsShort > 0) bits.push('credits short');
    return `${head}, ${bits.join(', ')}`;
  }
  const lines = [head];
  for (const p of c.parts) lines.push(`    needs ${p.name} ×${p.count} → ${p.how}`);
  for (const r of c.resources.slice(0, 6)) lines.push(`    short ${r.name}: need ${fmtN(r.need)}, have ${fmtN(r.have)} → ${r.how}`);
  if (c.creditsShort > 0) lines.push(`    short ${fmtN(c.creditsShort)} credits`);
  return lines.join('\n');
}



return { missing, howPart, howItem, buildClose, effort, closeToCompleting, describe };
})();
__m["core/insight.js"] = (function () {
// Views of the account: relics, mastery, foundry, what can be built right now.
const { count, relicsOf } = __m["core/account.js"];
const { resolve, partsMissing, resMissing, buildable } = __m["core/recipes.js"];
const { goalStatus } = __m["core/goals.js"];
const { blueprintForm } = __m["core/util.js"];

/** Lowercase blueprint/component paths of everything the goals still need. */
function neededParts(cat, acct, goals) {
  const set = new Set();
  const all = [];
  for (const g of goals) {
    const st = goalStatus(cat, acct, g);
    if (!st.done) for (const [u] of st.missingParts) { set.add(u.toLowerCase()); set.add(blueprintForm(u).toLowerCase()); }
    all.push(st);
  }
  return { needed: set, statuses: all };
}

function relicRows(cat, acct, needed) {
  const rows = [];
  for (const [path, cnt] of relicsOf(acct)) {
    const hit = cat.relic(path);
    if (!hit) continue;
    const [relic, refinement] = hit;
    const rewards = relic.rewards.map((r) => ({ unique: r.unique, name: cat.display(r.unique), rarity: r.rarity, useful: needed.has(r.unique.toLowerCase()) }));
    rows.push({ unique: path, name: relic.name, refinement, count: cnt, rewards, useful: rewards.some((r) => r.useful) });
  }
  rows.sort((a, b) => Number(b.useful) - Number(a.useful) || a.name.localeCompare(b.name) || a.refinement.localeCompare(b.refinement));
  return rows;
}

/** 'mastered' | 'owned' | 'missing' */
function masteryState(acct, unique, masteryXp) {
  if ((acct.xp.get(unique) || 0) >= masteryXp && masteryXp > 0) return 'mastered';
  if (acct.owned.has(unique)) return 'owned';
  return 'missing';
}

function masteryRows(cat, acct) {
  const rows = cat.items.map((it) => ({ unique: it.unique, name: it.name, category: it.category, mrReq: it.mr, state: masteryState(acct, it.unique, it.masteryXp) }));
  rows.sort((a, b) => a.name.localeCompare(b.name));
  return rows;
}

function foundryRows(cat, acct, now = Date.now()) {
  return acct.foundry.map((j) => {
    const result = cat.resultOfBlueprint(j.recipe) || j.recipe;
    return { name: cat.display(result), unique: result, remainingMs: j.doneMs - now };
  });
}

/** Blueprints held that could be built right now. */
function buildableNow(cat, acct) {
  const out = new Set();
  for (const [path, e] of acct.inv) {
    if (e.count === 0 || !path.includes('/Recipes/')) continue;
    const result = cat.resultOfBlueprint(path);
    if (!result) continue;
    const res = resolve(cat, acct, result, 1);
    if (!res) continue;
    const othersOk = partsMissing(res).every((p) => p.unique.toLowerCase() === result.toLowerCase());
    if (othersOk && resMissing(res).length === 0 && acct.credits >= res.credits) out.add(result);
  }
  return [...out].sort();
}



return { count, buildable, neededParts, relicRows, masteryState, masteryRows, foundryRows, buildableNow };
})();
__m["core/assistant.js"] = (function () {
// Ordis: deterministic answers computed from the account and the catalog.
const { count } = __m["core/account.js"];
const { resolve, partsMissing, resMissing, buildable } = __m["core/recipes.js"];
const { goalStatus } = __m["core/goals.js"];
const { closeToCompleting, describe, howPart, howItem } = __m["core/close.js"];
const { buildableNow, foundryRows, neededParts, relicRows, masteryRows } = __m["core/insight.js"];
const { blueprintForm, fmtN, fmtDuration } = __m["core/util.js"];

const has = (q, words) => words.some((w) => q.includes(w));

const HELP = [
  'Ordis can answer things like:',
  '• What am I close to completing?',
  '• What am I missing for Wisp Prime?',
  '• What can I build right now?',
  '• Which relics should I crack?',
  '• What are my goals?',
  '• How much Ferrite do I have?',
  '• What\'s my mastery progress? What\'s in the Foundry?',
].join('\n');

function buildReport(cat, acct, unique, name) {
  const res = resolve(cat, acct, unique, 1);
  if (!res) return `Ordis has no recipe on file for ${name}.`;
  const out = [];
  const parts = partsMissing(res);
  if (parts.length === 0) out.push(`For ${name}: every part is accounted for.`);
  else {
    out.push(`For ${name} you still need to obtain:`);
    for (const p of parts) out.push(`• ${cat.display(blueprintForm(p.unique))} ×${p.toObtain}: ${howPart(cat, acct, p.unique)}`);
  }
  const short = resMissing(res);
  if (short.length) {
    out.push('Resources short:');
    for (const r of short.slice(0, 12)) out.push(`• ${cat.display(r.unique)}: need ${fmtN(r.need)}, have ${fmtN(r.have)} (${howItem(cat, r.unique)})`);
  }
  if (acct.credits < res.credits) out.push(`Credits: ${fmtN(res.credits)} needed, you have ${fmtN(acct.credits)}.`);
  if (buildable(res, acct)) out.push('You have everything. Start the build in the Foundry.');
  return out.join('\n');
}

function answer(ctx, question) {
  const { cat, acct, goals } = ctx;
  const q = question.toLowerCase();
  if (!acct) return 'Operator, Ordis cannot see your inventory yet. Start Warframe with Overwolf running, and leave the Relay or Orbiter once so the game sends it.';
  if (has(q, ['hello', 'hi ordis', 'hey ordis']) || q.trim() === 'hi') return 'Operator! Ordis is online and fully operational. Ask away.';
  if (has(q, ['what can you', 'help me with', 'what do you do']) || q.trim() === 'help') return HELP;

  if (has(q, ['close to', 'closest', 'nearly', 'almost', 'next to finish', 'what can i finish', 'what am i close'])) {
    const list = closeToCompleting(cat, acct, goals, 4);
    if (!list.length) return 'Ordis cannot see anything you are partway through yet. Add a goal, or get a Prime part and ask again.';
    const detailed = has(q, ['how', 'where', 'detail', 'exactly', 'get']);
    const lines = list.map((x, i) => describe(x, detailed || i === 0));
    return `You are closest to finishing:\n${lines.join('\n')}${detailed ? '' : '\nAsk "how do I get them" for the full list of sources.'}`;
  }

  const mentioned = cat.mentioned(q, 3);
  if (has(q, ['missing', 'need', 'build', 'make ', 'require']) && !has(q, ['what can i build', 'can i build'])) {
    const item = cat.mentionedItem(q) || mentioned.map(([u]) => cat.item(u)).find(Boolean);
    if (item) return buildReport(cat, acct, item.unique, item.name);
    const hit = mentioned.find(([u]) => cat.recipe(u));
    if (hit) return buildReport(cat, acct, hit[0], hit[1]);
  }
  if (has(q, ['what can i build', 'can i build', 'ready to build'])) {
    const list = buildableNow(cat, acct);
    return list.length ? `You can build right now: ${list.map((u) => cat.display(u)).join(', ')}.` : 'Nothing is ready: Ordis found no blueprint you hold with all resources and credits available.';
  }
  if (has(q, ['relic'])) {
    const { needed } = neededParts(cat, acct, goals);
    const rows = relicRows(cat, acct, needed).filter((r) => r.useful);
    if (!rows.length) return 'None of your relics hold a part your goals need right now.';
    return `Relics worth cracking:\n${rows.slice(0, 8).map((r) => `• ${r.name} (${r.refinement}) ×${r.count}: ${r.rewards.filter((x) => x.useful).map((x) => x.name).join(', ')}`).join('\n')}`;
  }
  if (has(q, ['goal', 'progress']) && !q.includes('mastery')) {
    if (!goals.length) return 'You have no goals yet. Add one in the Goals tab, for example "Wisp Prime".';
    return goals.map((g) => `• ${g.name}: ${goalStatus(cat, acct, g).summary}`).join('\n');
  }
  if (has(q, ['mastery', 'mastered'])) {
    const rows = masteryRows(cat, acct);
    const m = rows.filter((r) => r.state === 'mastered').length, o = rows.filter((r) => r.state === 'owned').length;
    return `Mastery rank ${acct.mr}. ${m} of ${rows.length} items mastered, ${o} owned but not yet levelled.`;
  }
  if (has(q, ['foundry', 'building'])) {
    const rows = foundryRows(cat, acct);
    if (!rows.length) return 'Nothing is building in the Foundry.';
    return rows.map((r) => `• ${r.name}: ${r.remainingMs <= 0 ? 'ready to claim' : fmtDuration(r.remainingMs) + ' left'}`).join('\n');
  }
  if (has(q, ['credits', 'platinum', 'endo'])) {
    return `Credits ${fmtN(acct.credits)}, platinum ${fmtN(acct.platinum)}, endo ${fmtN(acct.endo)}.`;
  }
  if (has(q, ['how much', 'how many', 'do i have'])) {
    const hit = mentioned[0];
    if (hit) return `You have ${fmtN(count(acct, hit[0]))} × ${hit[1]}.`;
  }
  return `Ordis did not quite catch that, Operator.\n\n${HELP}`;
}

return { HELP, buildReport, answer };
})();
__m["ui/main.js"] = (function () {
// Main window. Everything is computed from the background window's state.
const { answer, HELP } = __m["core/assistant.js"];
const { closeToCompleting, describe } = __m["core/close.js"];
const { goalStatus } = __m["core/goals.js"];
const { neededParts, relicRows, masteryRows, foundryRows, buildableNow } = __m["core/insight.js"];
const { count } = __m["core/account.js"];
const { fmtN, fmtDuration } = __m["core/util.js"];

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

function errorBanner() {
  const errs = (bg && bg.state.startupErrors) || [];
  const old = document.getElementById('errs'); if (old) old.remove();
  if (!errs.length) return;
  const d = h('div', { id: 'errs', class: 'card bad' }, h('b', {}, 'Startup problems (send these to the author):'), h('pre', {}, errs.slice(0, 8).join('\n')));
  $('#view').prepend(d);
}

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
      table(['', ''], [['Ordis Nexus version', st.version], ['Warframe running', s.gameRunning ? 'yes' : 'no'], ['Game Overwolf sees', s.game ? `${s.game.title || '?'} (id ${s.game.id}, class ${s.game.classId}, running ${s.game.isRunning})` : 'none'], ['GEP features registered', s.featuresSet ? 'yes' : 'no'], ['Last error', s.lastError || '—'],
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
  if (!ready || (tab !== 'Settings' && tab !== 'Ordis' && !bg.state.acct)) { main.replaceChildren(tab === 'Settings' ? views.Settings() : needData(tab)); errorBanner(); return; }
  if (!bg.state.acct && (tab === 'Goals')) { main.replaceChildren(needData(tab)); return; }
  main.replaceChildren(views[tab]());
  errorBanner();
  if (keep && $('#q')) $('#q').focus();
}

function start() {
  let w = null;
  try { w = overwolf.windows.getMainWindow(); } catch (e) { document.getElementById('view').textContent = `Could not reach the background window: ${e && e.message ? e.message : e}`; return; }
  bg = w && w.ordis;
  if (!bg) { document.getElementById('view').textContent = 'The Ordis Nexus background window is not running (it failed to start). Open the Overwolf developer tools for the background window and send the error to the author.'; return; }
  bg.subscribe(() => { if (document.activeElement && ['INPUT', 'SELECT'].includes(document.activeElement.tagName) && tab !== 'Settings') { pill(); return; } draw(); });
  bg.catalog.then((c) => { cat = c; draw(); }, () => draw());
  draw();
  setInterval(draw, 30000);
}
start();

return {  };
})();
})();
