// Ordis Nexus Bridge (Overwolf background page)
//
// Listens to Warframe's Game Events Provider (GEP) and forwards everything to
// the Ordis Nexus desktop app listening on 127.0.0.1. Nothing leaves this PC.
//
// Flow, as documented by the research notes that came with the project:
//   set required features -> getInfo() -> onInfoUpdates2 for later updates.
// Warframe only includes `match_info.inventory` in some updates, so every update
// is forwarded and the desktop app keeps the ones that carry an inventory.

const VERSION = '0.1.0';
const WARFRAME_CLASS_ID = 8954; // Overwolf's id for Warframe; the title check below is a backstop
const ORDIS = 'http://127.0.0.1:17642';
const FEATURES = ['match_info', 'game_info'];
const POLL_MS = 60 * 1000;      // gentle extra getInfo() while the game runs
const HEARTBEAT_MS = 15 * 1000;

let gameRunning = false;
let featuresSet = false;
let lastInventory = null;       // skip re-sending an identical snapshot
let pollTimer = null;

function log(...a) { console.log('[ordis-bridge]', ...a); }

function post(kind, data) {
  const body = JSON.stringify({ kind: kind, ts: Date.now(), data: data });
  return fetch(ORDIS + '/gep', {
    method: 'POST',
    headers: { 'Content-Type': 'text/plain' },
    body: body
  }).catch(function () { /* Ordis Nexus is not running: that is fine */ });
}

function sendInfo(info) {
  if (!info) return;
  // De-duplicate identical inventory snapshots to avoid re-sending ~600 KB.
  let inv = null;
  try {
    const mi = (info.info && info.info.match_info) || info.match_info;
    if (mi && typeof mi.inventory === 'string') inv = mi.inventory;
  } catch (e) { /* ignore */ }
  if (inv !== null) {
    if (inv === lastInventory) return;
    lastInventory = inv;
  }
  post('info', info);
}

function isWarframe(g) {
  if (!g) return false;
  if (g.classId === WARFRAME_CLASS_ID || Math.floor((g.id || 0) / 10) === WARFRAME_CLASS_ID) return true;
  return /warframe/i.test(g.title || g.name || '');
}

function getInfo() {
  overwolf.games.events.getInfo(function (res) {
    if (res && res.success !== false) sendInfo(res);
    else log('getInfo failed', res);
  });
}

function setFeatures(attempt) {
  overwolf.games.events.setRequiredFeatures(FEATURES, function (res) {
    if (res && res.success) {
      featuresSet = true;
      log('features set', res);
      getInfo();
    } else if (attempt < 30) {
      // The game is often not ready for a few seconds after launch.
      setTimeout(function () { setFeatures(attempt + 1); }, 2000);
    } else {
      log('could not set required features', res);
      post('event', { name: 'bridge_error', detail: 'setRequiredFeatures failed' });
    }
  });
}

function gameStarted() {
  if (gameRunning) return;
  gameRunning = true;
  featuresSet = false;
  lastInventory = null;
  log('Warframe detected');
  post('status', { gameRunning: true, version: VERSION });
  setFeatures(0);
  clearInterval(pollTimer);
  pollTimer = setInterval(function () { if (featuresSet) getInfo(); }, POLL_MS);
}

function gameStopped() {
  if (!gameRunning) return;
  gameRunning = false;
  featuresSet = false;
  clearInterval(pollTimer);
  log('Warframe closed');
  post('status', { gameRunning: false, version: VERSION });
}

overwolf.games.events.onError.addListener(function (e) { log('GEP error', e); });
overwolf.games.events.onInfoUpdates2.addListener(function (u) { sendInfo(u); });
overwolf.games.events.onNewEvents.addListener(function (e) { post('event', e); });

overwolf.games.onGameInfoUpdated.addListener(function (u) {
  const g = u && u.gameInfo;
  if (!u || !isWarframe(g)) return;
  if (g.isRunning && (u.runningChanged || u.gameChanged)) gameStarted();
  else if (!g.isRunning) gameStopped();
});

overwolf.games.getRunningGameInfo(function (g) {
  if (g && g.isRunning && isWarframe(g)) gameStarted();
});

setInterval(function () {
  post('status', { gameRunning: gameRunning, version: VERSION });
}, HEARTBEAT_MS);

log('Ordis Nexus Bridge ' + VERSION + ' loaded');
