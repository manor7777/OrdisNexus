// Bundles the ES-module sources into two classic scripts (no import/export at runtime),
// because they are the most reliable thing to load inside an Overwolf window.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const read = (p) => fs.readFileSync(path.join(root, p), 'utf8');

function bundle(entry, out) {
  const order = [];
  const seen = new Set();
  const norm = (from, spec) => path.posix.normalize(path.posix.join(path.posix.dirname(from), spec));
  function visit(file) {
    if (seen.has(file)) return;
    seen.add(file);
    const src = read(file);
    for (const m of src.matchAll(/^import\s+[^;]*?from\s+['"](\.[^'"]+)['"];?/gm)) visit(norm(file, m[1]));
    order.push(file);
  }
  visit(entry);
  let code = '"use strict";\n(function () {\nconst __m = {};\n';
  for (const file of order) {
    let src = read(file);
    const exports = [];
    src = src.replace(/^import\s+\{([^}]*)\}\s+from\s+['"](\.[^'"]+)['"];?/gm, (_, names, spec) => {
      const target = norm(file, spec);
      const bind = names.split(',').map((s) => s.trim()).filter(Boolean).map((s) => s.replace(/\s+as\s+/, ': ')).join(', ');
      return `const { ${bind} } = __m[${JSON.stringify(target)}];`;
    });
    src = src.replace(/^export\s+\{([^}]*)\};?/gm, (_, names) => { names.split(',').map((s) => s.trim()).filter(Boolean).forEach((n) => exports.push(n)); return ''; });
    src = src.replace(/^export\s+(async\s+function|function|class|const|let)\s+([A-Za-z0-9_$]+)/gm, (_, kind, name) => { exports.push(name); return `${kind} ${name}`; });
    if (/^\s*(import|export)\s/m.test(src)) throw new Error(`unhandled import/export in ${file}`);
    code += `__m[${JSON.stringify(file)}] = (function () {\n${src}\nreturn { ${[...new Set(exports)].join(', ')} };\n})();\n`;
  }
  code += '})();\n';
  fs.mkdirSync(path.join(root, 'bundle'), { recursive: true });
  fs.writeFileSync(path.join(root, out), code);
  return code.length;
}

console.log('background', bundle('background.js', 'bundle/background.js'));
console.log('ui', bundle('ui/main.js', 'bundle/ui.js'));
