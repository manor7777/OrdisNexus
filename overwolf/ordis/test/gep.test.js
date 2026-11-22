import test from 'node:test';
import assert from 'node:assert/strict';
import { extractInventory } from '../core/gep.js';

const full = { PlayerLevel: 3, RegularCredits: 5, MiscItems: [] };

test('onInfoUpdates2 shape', () => {
  const r = extractInventory({ feature: 'match_info', info: { feature: 'match_info', category: 'match_info', key: 'inventory', value: JSON.stringify(full) } });
  assert.ok(r.hasInventory && r.complete);
});
test('getInfo shape', () => {
  const r = extractInventory({ info: { match_info: { inventory: JSON.stringify(full) } } });
  assert.ok(r.complete && r.root.PlayerLevel === 3);
});
test('partial inventory is flagged incomplete', () => {
  const r = extractInventory({ info: { match_info: { inventory: JSON.stringify({ FusionPoints: 95 }) } } });
  assert.ok(r.hasInventory && !r.complete);
  assert.deepEqual(r.keys, ['FusionPoints']);
});
test('no inventory, bad json, junk', () => {
  assert.ok(!extractInventory({ info: { game_info: { x: 1 } } }).hasInventory);
  assert.ok(!extractInventory({ info: { match_info: { inventory: '{oops' } } }).complete);
  assert.ok(!extractInventory(null).hasInventory);
});
