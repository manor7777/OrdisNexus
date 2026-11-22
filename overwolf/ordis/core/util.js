// Small helpers shared by the engine. Pure functions only (no DOM, no Overwolf).

export function swapSuffix(unique, from, to) {
  const n = unique.length;
  if (n >= from.length && unique.slice(n - from.length).toLowerCase() === from.toLowerCase()) {
    const tail = unique.slice(n - from.length);
    const t = tail === tail.toLowerCase() ? to.toLowerCase() : to; // keep lowercase paths lowercase
    return unique.slice(0, n - from.length) + t;
  }
  return unique;
}

export const blueprintForm = (u) => swapSuffix(u, 'Component', 'Blueprint');
export const componentForm = (u) => swapSuffix(u, 'Blueprint', 'Component');

/** `/Lotus/Types/Items/MiscItems/OrokinCatalyst` -> `Orokin Catalyst` */
export function prettyName(path) {
  const last = path.split('/').pop() || path;
  return prettyRaw(last);
}

export function prettyRaw(last) {
  let out = '';
  for (let i = 0; i < last.length; i++) {
    const c = last[i];
    if (i > 0 && c !== c.toLowerCase() && c === c.toUpperCase()) {
      const prev = last[i - 1];
      const next = last[i + 1];
      const nextLower = next !== undefined && next === next.toLowerCase() && next !== next.toUpperCase();
      const prevLower = prev === prev.toLowerCase() && prev !== prev.toUpperCase();
      const prevUpper = prev === prev.toUpperCase() && prev !== prev.toLowerCase();
      const prevDigit = prev >= '0' && prev <= '9';
      if (prevLower || prevDigit || (prevUpper && nextLower)) out += ' ';
    }
    out += c;
  }
  return out;
}

export const fmtN = (n) => Math.round(n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');

export function fmtDuration(ms) {
  if (ms <= 0) return 'now';
  let s = Math.floor(ms / 1000);
  const d = Math.floor(s / 86400); s %= 86400;
  const h = Math.floor(s / 3600); s %= 3600;
  const m = Math.floor(s / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}
