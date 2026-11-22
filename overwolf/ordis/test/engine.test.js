import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import zlib from 'node:zlib';
import { Catalog } from '../core/catalog.js';
import { parseAccount, looksComplete, count, gained } from '../core/account.js';
import { resolve, partsMissing, buildable } from '../core/recipes.js';
import { goalStatus } from '../core/goals.js';
import { closeToCompleting, describe } from '../core/close.js';
import { answer } from '../core/assistant.js';
import { relicRows, neededParts, masteryRows, buildableNow } from '../core/insight.js';
import { swapSuffix, prettyName } from '../core/util.js';

const realCatalog = () => new Catalog(JSON.parse(zlib.gunzipSync(fs.readFileSync(new URL('../data/catalog.json.gz', import.meta.url))).toString()));

const tiny = () => new Catalog({
  names: {
    '/lotus/types/recipes/warframerecipes/wispprimesystemsblueprint': 'Wisp Prime Systems Blueprint',
    '/lotus/types/items/miscitems/ferrite': 'Ferrite',
  },
  icons: {}, primes: [],
  recipes: {
    '/lotus/powersuits/wisp/wispprime': { r: '/Lotus/Powersuits/Wisp/WispPrime', b: '/Lotus/Types/Recipes/WarframeRecipes/WispPrimeBlueprint', c: 25000, t: 259200, n: 1,
      i: [['/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsComponent', 1], ['/Lotus/Types/Items/MiscItems/OrokinCell', 5]] },
    '/lotus/types/recipes/warframerecipes/wispprimesystemscomponent': { r: '/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsComponent', b: '/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsBlueprint', c: 15000, t: 43200, n: 1,
      i: [['/Lotus/Types/Items/MiscItems/Ferrite', 100], ['/Lotus/Types/Items/MiscItems/Circuits', 50]] },
  },
  items: [{ u: '/Lotus/Powersuits/Wisp/WispPrime', n: 'Wisp Prime', c: 'Suits', m: 0, x: 900000 }],
  relics: { '/lotus/types/game/projections/t4voidprojectionwispprimea': { n: 'Axi W1', rw: [['/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsBlueprint', 'uncommon', 1]] } },
  slugs: {}, res: {}, drops: {},
});

test('util', () => {
  assert.equal(swapSuffix('/a/FooComponent', 'Component', 'Blueprint'), '/a/FooBlueprint');
  assert.equal(swapSuffix('/a/foocomponent', 'Component', 'Blueprint'), '/a/fooblueprint');
  assert.equal(prettyName('/Lotus/Types/Items/MiscItems/OrokinCatalyst'), 'Orokin Catalyst');
});

test('account parsing', () => {
  const a = parseAccount({
    PlayerLevel: 24, RegularCredits: 1000, PremiumCredits: 5, PremiumCreditsFree: 2, FusionPoints: 95,
    MiscItems: [{ ItemType: '/Lotus/Types/Items/MiscItems/Ferrite', ItemCount: 9 }],
    Suits: [{ ItemType: '/Lotus/Powersuits/Wisp/Wisp' }],
    XPInfo: [{ ItemType: '/Lotus/Powersuits/Wisp/Wisp', XP: 900000 }],
    PendingRecipes: [{ ItemType: '/Lotus/Types/Recipes/WarframeRecipes/WispPrimeBlueprint', CompletionDate: { $date: { $numberLong: '1700000000000' } } }],
  });
  assert.deepEqual([a.mr, a.credits, a.platinum, a.endo], [24, 1000, 7, 95]);
  assert.equal(count(a, '/lotus/types/items/miscitems/FERRITE'), 9);
  assert.ok(a.owned.has('/Lotus/Powersuits/Wisp/Wisp'));
  assert.equal(a.foundry[0].doneMs, 1700000000000);
  assert.ok(!a.inv.has('/Lotus/Powersuits/Wisp/Wisp') === false);
});

test('a partial inventory is not complete', () => {
  assert.ok(!looksComplete({ FusionPoints: 95, MiscItems: [] }));
  assert.ok(looksComplete({ PlayerLevel: 1, RegularCredits: 0, MiscItems: [] }));
});

test('resolve: missing part, resources and credits', () => {
  const cat = tiny();
  const acct = parseAccount({ MiscItems: [{ ItemType: '/Lotus/Types/Items/MiscItems/OrokinCell', ItemCount: 2 }] });
  const r = resolve(cat, acct, '/Lotus/Powersuits/Wisp/WispPrime');
  assert.equal(r.parts[0].toObtain, 1);
  const ferrite = r.resources.find((x) => x.unique.endsWith('Ferrite'));
  assert.deepEqual([ferrite.need, ferrite.have], [100, 0]);
  assert.equal(r.credits, 40000);
  assert.ok(!buildable(r, acct));
});

test('resolve: blueprint in hand counts as having the part', () => {
  const cat = tiny();
  const acct = parseAccount({ Recipes: [{ ItemType: '/Lotus/Types/Recipes/WarframeRecipes/WispPrimeSystemsBlueprint', ItemCount: 1 }] });
  const r = resolve(cat, acct, '/Lotus/Powersuits/Wisp/WispPrime');
  assert.equal(partsMissing(r).length, 0);
  assert.ok(r.resources.some((x) => x.unique.endsWith('Circuits')));
});

test('goals', () => {
  const cat = tiny();
  const acct = parseAccount({ PlayerLevel: 12, MiscItems: [{ ItemType: '/Lotus/Types/Items/MiscItems/Ferrite', ItemCount: 40 }] });
  const build = goalStatus(cat, acct, { kind: 'build', target: '/Lotus/Powersuits/Wisp/WispPrime', name: 'Wisp Prime', count: 1 });
  assert.ok(!build.done);
  assert.ok(build.missingParts.length > 0);
  const collect = goalStatus(cat, acct, { kind: 'collect', target: '/Lotus/Types/Items/MiscItems/Ferrite', count: 100 });
  assert.equal(collect.summary, '40 / 100');
  assert.ok(goalStatus(cat, acct, { kind: 'mastery', count: 10 }).done);
});

test('close to completing explains where to get missing parts', () => {
  const cat = tiny();
  const acct = parseAccount({ PlayerLevel: 5, RegularCredits: 1e6, MiscItems: [{ ItemType: '/Lotus/Types/Items/MiscItems/OrokinCell', ItemCount: 9 }] });
  const goals = [{ kind: 'build', target: '/Lotus/Powersuits/Wisp/WispPrime', name: 'Wisp Prime', count: 1 }];
  const list = closeToCompleting(cat, acct, goals);
  assert.equal(list.length, 1);
  assert.ok(describe(list[0], true).includes('Axi W1'));
});

test('the real bundled catalog loads and works end to end', () => {
  const cat = realCatalog();
  assert.ok(cat.primes.length > 100, `primes ${cat.primes.length}`);
  assert.ok(cat.relics.size > 100);
  const prime = cat.primes.find((p) => p.parts.length >= 3);
  assert.ok(prime);
  // own just one part of this prime, as a blueprint
  const part = prime.parts.find((p) => /Component$/.test(p.unique));
  const acct = parseAccount({ PlayerLevel: 20, RegularCredits: 100000, MiscItems: [{ ItemType: part.unique, ItemCount: 1 }] });
  const list = closeToCompleting(cat, acct, [], 200);
  assert.ok(list.length > 0, 'a held part makes the prime show up');
  const text = list.map((c) => describe(c, true)).join('\n');
  assert.ok(text.length > 0);
  const reply = answer({ cat, acct, goals: [] }, 'what am I close to completing?');
  assert.ok(reply.startsWith('You are closest'), reply);
  assert.ok(answer({ cat, acct, goals: [] }, 'how much credits do I have').includes('100,000'));
});

test('gained items between snapshots', () => {
  const a = parseAccount({ MiscItems: [{ ItemType: '/x/Ferrite', ItemCount: 5 }] });
  const b = parseAccount({ MiscItems: [{ ItemType: '/x/Ferrite', ItemCount: 9 }, { ItemType: '/x/Rubedo', ItemCount: 2 }] });
  assert.deepEqual(gained(a, b), [['/x/Ferrite', 4], ['/x/Rubedo', 2]]);
});

// Only runs when a real GEP sample is supplied locally (it is private account data, never committed).
test('real GEP inventory sample', { skip: !process.env.ORDIS_SAMPLE }, () => {
  const cat = realCatalog();
  const doc = JSON.parse(fs.readFileSync(process.env.ORDIS_SAMPLE, 'utf8'));
  const root = JSON.parse(doc.info.match_info.inventory);
  assert.ok(looksComplete(root));
  const acct = parseAccount(root);
  console.log('MR', acct.mr, 'credits', acct.credits, 'plat', acct.platinum, 'endo', acct.endo, 'items', acct.inv.size, 'foundry', acct.foundry.length, 'owned', acct.owned.size, 'xp', acct.xp.size);
  assert.ok(acct.mr > 0 && acct.credits > 0 && acct.inv.size > 100);
  const rows = masteryRows(cat, acct);
  console.log('mastered', rows.filter((r) => r.state === 'mastered').length, 'owned', rows.filter((r) => r.state === 'owned').length, 'of', rows.length);
  console.log('relic rows', relicRows(cat, acct, new Set()).length, 'buildable', buildableNow(cat, acct).length);
  const t0 = Date.now();
  const close = closeToCompleting(cat, acct, [], 5);
  console.log('close-to-completing took', Date.now() - t0, 'ms');
  console.log(close.map((c) => describe(c, false)).join('\n'));
  console.log(answer({ cat, acct, goals: [] }, 'what am i close to completing? how do i get them'));
});
