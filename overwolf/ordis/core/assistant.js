// Ordis: deterministic answers computed from the account and the catalog.
import { count } from './account.js';
import { resolve, partsMissing, resMissing, buildable } from './recipes.js';
import { goalStatus } from './goals.js';
import { closeToCompleting, describe, howPart, howItem } from './close.js';
import { buildableNow, foundryRows, neededParts, relicRows, masteryRows } from './insight.js';
import { blueprintForm, fmtN, fmtDuration } from './util.js';

const has = (q, words) => words.some((w) => q.includes(w));

export const HELP = [
  'Ordis can answer things like:',
  '• What am I close to completing?',
  '• What am I missing for Wisp Prime?',
  '• What can I build right now?',
  '• Which relics should I crack?',
  '• What are my goals?',
  '• How much Ferrite do I have?',
  '• What\'s my mastery progress? What\'s in the Foundry?',
].join('\n');

export function buildReport(cat, acct, unique, name) {
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

export function answer(ctx, question) {
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
