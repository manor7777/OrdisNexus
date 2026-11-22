// Pulls the account inventory out of Overwolf Game Events Provider messages.
// The same JSON document shows up in a few shapes depending on the call:
//   onInfoUpdates2: { feature, category, key: 'inventory', value: '<json string>' }
//   getInfo():      { info: { match_info: { inventory: '<json string>' } } }
import { looksComplete } from './account.js';

export function findInventory(v, depth = 0) {
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
export function extractInventory(msg) {
  const raw = findInventory(msg);
  if (raw === undefined) return { hasInventory: false, complete: false, root: null, keys: [] };
  let root = raw;
  if (typeof raw === 'string') {
    try { root = JSON.parse(raw); } catch { return { hasInventory: true, complete: false, root: null, keys: [] }; }
  }
  const keys = root && typeof root === 'object' ? Object.keys(root).slice(0, 40) : [];
  return { hasInventory: true, complete: looksComplete(root), root, keys };
}
