// "What am I close to completing?" Ranks things you could finish soon and says
// exactly what is missing and where to get it, using only the stored catalog.
import { relicsOf } from './account.js';
import { resolve, partsMissing, resMissing, missing, buildable, fraction } from './recipes.js';
import { blueprintForm, fmtN } from './util.js';

const rarityWord = (r) => ({ c: 'common', u: 'uncommon', r: 'rare' }[(r || '')[0]?.toLowerCase()] || r);

/** How to get a Prime part: relics that drop it, marking ones you already hold. */
export function howPart(cat, acct, partUnique) {
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
export function howItem(cat, unique) {
  const hint = cat.resourceHint(unique);
  if (hint) return hint;
  const drops = cat.dropsFor(cat.display(unique));
  if (drops.length === 0) return 'no source on file';
  return `drops from ${drops.slice(0, 3).map((d) => `${d.place} (${d.chance.toFixed(1)}%)`).join('; ')}`;
}

export function buildClose(cat, acct, unique, name, isGoal) {
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
export const effort = (c) => c.parts.reduce((a, p) => a + p.count, 0) * 100 + c.resources.length * 8 + (c.creditsShort > 0 ? 5 : 0);

/** Unfinished Build goals plus every Prime you hold at least one part of, closest first. */
export function closeToCompleting(cat, acct, goals, limit = 5) {
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

export function describe(c, detailed) {
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

export { missing };
