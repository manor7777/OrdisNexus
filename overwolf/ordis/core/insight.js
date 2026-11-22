// Views of the account: relics, mastery, foundry, what can be built right now.
import { count, relicsOf } from './account.js';
import { resolve, partsMissing, resMissing, buildable } from './recipes.js';
import { goalStatus } from './goals.js';
import { blueprintForm } from './util.js';

/** Lowercase blueprint/component paths of everything the goals still need. */
export function neededParts(cat, acct, goals) {
  const set = new Set();
  const all = [];
  for (const g of goals) {
    const st = goalStatus(cat, acct, g);
    if (!st.done) for (const [u] of st.missingParts) { set.add(u.toLowerCase()); set.add(blueprintForm(u).toLowerCase()); }
    all.push(st);
  }
  return { needed: set, statuses: all };
}

export function relicRows(cat, acct, needed) {
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
export function masteryState(acct, unique, masteryXp) {
  if ((acct.xp.get(unique) || 0) >= masteryXp && masteryXp > 0) return 'mastered';
  if (acct.owned.has(unique)) return 'owned';
  return 'missing';
}

export function masteryRows(cat, acct) {
  const rows = cat.items.map((it) => ({ unique: it.unique, name: it.name, category: it.category, mrReq: it.mr, state: masteryState(acct, it.unique, it.masteryXp) }));
  rows.sort((a, b) => a.name.localeCompare(b.name));
  return rows;
}

export function foundryRows(cat, acct, now = Date.now()) {
  return acct.foundry.map((j) => {
    const result = cat.resultOfBlueprint(j.recipe) || j.recipe;
    return { name: cat.display(result), unique: result, remainingMs: j.doneMs - now };
  });
}

/** Blueprints held that could be built right now. */
export function buildableNow(cat, acct) {
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

export { count, buildable };
