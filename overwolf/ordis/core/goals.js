// Goals: build an item, collect N of something, or reach a mastery rank.
import { count } from './account.js';
import { resolve, partsMissing, resMissing, missing, buildable, fraction } from './recipes.js';

export function goalStatus(cat, acct, goal) {
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
