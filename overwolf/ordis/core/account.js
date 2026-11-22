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

import { prettyRaw } from './util.js';

export function categoryName(key) {
  return CATEGORY[key] || prettyRaw(key);
}

const num = (v) => (typeof v === 'number' && isFinite(v) ? Math.max(0, Math.floor(v)) : 0);

/** Mongo-style dates: {"$date":{"$numberLong":"170..."}} or {"$date":170...} */
export function jsonMs(v) {
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
export function looksComplete(root) {
  return !!root && typeof root === 'object' && ['PlayerLevel', 'RegularCredits', 'MiscItems'].every((k) => root[k] !== undefined);
}

export function parseAccount(root) {
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

export const count = (acct, unique) => acct.lower.get(unique.toLowerCase()) || 0;
export const isEmpty = (acct) => acct.inv.size === 0;

/** Relic items: [path, copies] */
export function relicsOf(acct) {
  const out = [];
  for (const [k, e] of acct.inv) if (k.includes('/Projections/') && k.includes('VoidProjection')) out.push([k, e.count]);
  return out;
}

/** Items gained between two accounts: [[path, delta]] (positive only). */
export function gained(before, after) {
  const out = [];
  for (const [p, e] of after.inv) {
    const d = e.count - (before.inv.get(p)?.count || 0);
    if (d > 0) out.push([p, d]);
  }
  return out.sort((a, b) => b[1] - a[1]);
}
