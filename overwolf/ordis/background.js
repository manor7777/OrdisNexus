// Background window: always running. Registers for Warframe game events, keeps the
// latest complete inventory, and shares state with the main window.
import { parseAccount } from './core/account.js';
import { extractInventory } from './core/gep.js';
import { loadCatalogFromUrl } from './core/catalog.js';

const startupErrors = [];
window.addEventListener('error', (e) => startupErrors.push(String(e.message || e)));
window.addEventListener('unhandledrejection', (e) => startupErrors.push(String((e.reason && e.reason.message) || e.reason)));
const safe = (what, fn) => { try { fn(); } catch (e) { startupErrors.push(`${what}: ${e && e.message ? e.message : e}`); } };

const GAME_ID = 8954; // Warframe (verify in Overwolf's game list if nothing arrives)
const FEATURES = ['match_info', 'game_info'];
const KEY = { inv: 'ordis.inventory', meta: 'ordis.meta', goals: 'ordis.goals', settings: 'ordis.settings' };
const store = {
  get(k, d) { try { const v = localStorage.getItem(k); return v === null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); return true; } catch { return false; } },
};

const state = {
  version: '0.4.1',
  acct: null,
  updatedAt: 0,
  goals: store.get(KEY.goals, []),
  settings: store.get(KEY.settings, { name: '' }),
  status: { gameRunning: false, featuresSet: false, lastError: '', messages: 0, withInventory: 0, partial: 0, lastPartialKeys: [], lastKeys: [], recent: [] },
  catalogError: '',
  startupErrors,
};
const listeners = new Set();
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* a closed window */ } });

const catalog = loadCatalogFromUrl('data/catalog.json.gz').catch((e) => { state.catalogError = String(e); notify(); throw e; });

function adopt(root, ts) {
  state.acct = parseAccount(root);
  state.updatedAt = ts;
}

// restore the last complete inventory from the previous session
(function restore() {
  const root = store.get(KEY.inv, null);
  if (root) { try { adopt(root, store.get(KEY.meta, {}).updatedAt || 0); } catch { /* ignore */ } }
})();

function log(kind, data) {
  const st = state.status;
  st.messages++;
  st.recent.unshift({ at: Date.now(), kind, summary: JSON.stringify(data).slice(0, 300) });
  st.recent.length = Math.min(st.recent.length, 25);
}

function handle(kind, msg) {
  log(kind, msg);
  const r = extractInventory(msg);
  const st = state.status;
  if (r.hasInventory) {
    st.withInventory++;
    st.lastKeys = r.keys;
    if (r.complete) {
      adopt(r.root, Date.now());
      store.set(KEY.inv, r.root);
      store.set(KEY.meta, { updatedAt: state.updatedAt });
    } else {
      st.partial++;
      st.lastPartialKeys = r.keys;
    }
  }
  notify();
}

function setFeatures(attempt = 0) {
  overwolf.games.events.setRequiredFeatures(FEATURES, (res) => {
    if (res && res.success) {
      state.status.featuresSet = true; state.status.lastError = '';
      overwolf.games.events.getInfo((info) => handle('getInfo', info));
    } else {
      state.status.featuresSet = false;
      state.status.lastError = `setRequiredFeatures: ${(res && (res.error || res.reason)) || 'failed'}`;
      if (attempt < 30) setTimeout(() => setFeatures(attempt + 1), 2000);
    }
    notify();
  });
}

function onGame(info) {
  const running = !!(info && info.isRunning && (Math.floor((info.id || 0) / 10) === GAME_ID || /warframe/i.test(info.title || '')));
  if (running !== state.status.gameRunning) {
    state.status.gameRunning = running;
    if (running) setFeatures();
    notify();
  }
}

safe('onInfoUpdates2', () => overwolf.games.events.onInfoUpdates2.addListener((m) => handle('info', m)));
safe('onNewEvents', () => overwolf.games.events.onNewEvents.addListener((m) => { log('event', m); notify(); }));
safe('onGameInfoUpdated', () => overwolf.games.onGameInfoUpdated.addListener((e) => onGame(e && e.gameInfo)));
safe('getRunningGameInfo', () => overwolf.games.getRunningGameInfo((info) => onGame(info)));
setInterval(() => { if (state.status.gameRunning) overwolf.games.events.getInfo((i) => handle('getInfo', i)); }, 60000);

// window handling
function showMain() { overwolf.windows.obtainDeclaredWindow('main', (r) => { if (r.success) overwolf.windows.restore(r.window.id, () => {}); }); }
safe('onAppLaunchTriggered', () => overwolf.extensions.onAppLaunchTriggered.addListener((e) => { if (!e || e.origin !== 'gamelaunchevent') showMain(); }));
safe('hotkeys', () => overwolf.settings.hotkeys.onPressed.addListener((e) => {
  if (e.name !== 'toggle_main') return;
  overwolf.windows.obtainDeclaredWindow('main', (r) => {
    if (!r.success) return;
    overwolf.windows.getWindowState(r.window.id, (s) => {
      const st = s.window_state_ex || s.window_state;
      if (st === 'normal' || st === 'maximized') overwolf.windows.minimize(r.window.id, () => {}); else overwolf.windows.restore(r.window.id, () => {});
    });
  });
}));

// API for the main window (overwolf.windows.getMainWindow() returns this window)
window.ordis = {
  state, catalog,
  subscribe(f) { listeners.add(f); return () => listeners.delete(f); },
  saveGoals(g) { state.goals = g; store.set(KEY.goals, g); notify(); },
  saveSettings(s) { state.settings = s; store.set(KEY.settings, s); notify(); },
  forgetInventory() { state.acct = null; state.updatedAt = 0; try { localStorage.removeItem(KEY.inv); } catch { /* ignore */ } notify(); },
  refresh() { overwolf.games.events.getInfo((i) => handle('getInfo', i)); },
};
safe('setFeatures', () => setFeatures());
// The launch event can fire before this script runs, so open the window ourselves
// unless Overwolf started us silently because the game launched.
if (!/gamelaunchevent/.test(window.location.href)) setTimeout(showMain, 300);
