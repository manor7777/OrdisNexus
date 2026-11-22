// What building an item needs, given what the account owns.
import { count } from './account.js';

const MAX_DEPTH = 4;

export function resolve(cat, acct, target, qty = 1) {
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

export const partsMissing = (r) => r.parts.filter((p) => p.toObtain > 0);
export const resMissing = (r) => r.resources.filter((x) => x.need > x.have);
export const missing = (x) => Math.max(0, x.need - x.have);
export const buildable = (r, acct) => partsMissing(r).length === 0 && resMissing(r).length === 0 && acct.credits >= r.credits;

/** 0..1, averaged over parts and resource totals. */
export function fraction(r) {
  let total = 0, n = 0;
  for (const p of r.parts) { total += Math.max(0, p.need - p.toObtain) / Math.max(1, p.need); n++; }
  for (const x of r.resources) { total += Math.min(x.have, x.need) / Math.max(1, x.need); n++; }
  return n === 0 ? 1 : total / n;
}
