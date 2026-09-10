import path from 'node:path';

const sourceExtensions = new Set(('.rs .kt .kts .java .ts .tsx .js .jsx .mjs .cjs .ps1 .psm1 .sh ' +
  '.py .go .swift .cs .c .cc .cpp .h .hpp .css .scss .sass .less .html .vue .svelte .move .sql .lua .rb .php').split(' '));
const skipped = new Set(['.git', '.gradle', '.idea', '.next', '.nuxt', '.venv', '.runtime',
  'build', 'dist', 'node_modules', 'out', 'target', 'vendor', 'generated', '__pycache__']);
const limits = { entry: 500, source: 800, helper: 600, schema: 1000, test: 1000, document: 800 };
export const CONFIG = '.modularity.json';
export const BASELINE = '.modularity-baseline.json';

export function assertKeys(value, keys, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(label + ' must be an object');
  for (const key of Object.keys(value)) if (!keys.includes(key)) throw new Error(label + ': unknown key ' + key);
}

function positive(value, label) {
  if (!Number.isSafeInteger(value) || value < 1) throw new Error(label + ' must be a positive integer');
}

export function glob(pattern) {
  if (typeof pattern !== 'string' || !pattern || pattern.includes('\\') ||
      pattern.startsWith('/') || pattern.includes(':') || pattern.split('/').includes('..')) {
    throw new Error('Expected a relative forward-slash glob: ' + pattern);
  }
  let result = '^';
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === '*' && pattern[i + 1] === '*') {
      i++;
      if (pattern[i + 1] === '/') { i++; result += '(?:.*/)?'; }
      else result += '.*';
    } else if (c === '*') result += '[^/]*';
    else result += '\\^$+?.()|{}[]'.includes(c) ? '\\' + c : c;
  }
  return new RegExp(result + '$');
}

export function parseConfig(text) {
  const config = text == null ? { version: 1 } : JSON.parse(text);
  assertKeys(config, ['version', 'roles', 'exclude'], CONFIG);
  if (config.version !== 1) throw new Error('Unsupported configuration version');
  for (const key of ['roles', 'exclude']) {
    if (config[key] != null && !Array.isArray(config[key])) throw new Error(key + ' must be an array');
  }
  const roles = (config.roles ?? []).map(rule => {
    assertKeys(rule, ['pattern', 'role', 'maxLines', 'maxBytes', 'reason'], 'role rule');
    if (!Object.hasOwn(limits, rule.role)) throw new Error('Unknown role ' + rule.role);
    if (rule.maxLines != null) positive(rule.maxLines, 'maxLines');
    if (rule.maxBytes != null) positive(rule.maxBytes, 'maxBytes');
    if ((rule.maxLines != null || rule.maxBytes != null) && !rule.reason?.trim()) {
      throw new Error('Custom limits require a reason');
    }
    return { ...rule, matcher: glob(rule.pattern) };
  });
  const exclude = (config.exclude ?? []).map(rule => {
    assertKeys(rule, ['pattern', 'reason'], 'exclude rule');
    if (typeof rule.reason !== 'string' || !rule.reason.trim()) throw new Error('Exclusion requires a reason');
    return glob(rule.pattern);
  });
  return { roles, exclude };
}

export function classify(file, config) {
  if (file.split('/').some(part => skipped.has(part)) || config.exclude.some(rx => rx.test(file))) return null;
  const name = path.posix.basename(file);
  const lower = name.toLowerCase();
  const ext = path.posix.extname(lower);
  if (!sourceExtensions.has(ext) && ext !== '.md' && ext !== '.mdx') return null;
  let role = 'source';
  if (ext === '.md' || ext === '.mdx') role = 'document';
  else if (/^(main|router|app)\.(rs|[cm]?js|jsx|ts|tsx|py|go)$/.test(lower) || name === 'MainActivity.kt') role = 'entry';
  else if (/(^|\/)(tests?|__tests__)\//.test(file) ||
      /(^test[-_]|^tests\.|[-_]tests?\.[^.]+$|\.(test|spec)\.[^.]+$|Test\.(kt|java)$)/.test(name)) role = 'test';
  else if (/(^|[-_.])(types?|schema)([-_.]|$)/.test(lower) || lower.endsWith('.d.ts')) role = 'schema';
  else if (/(helper|util|^common\.)/.test(lower)) role = 'helper';
  const rule = config.roles.find(item => item.matcher.test(file));
  role = rule?.role ?? role;
  return { role, maxLines: rule?.maxLines ?? limits[role], maxBytes: rule?.maxBytes ?? (role === 'document' ? 50000 : 128000) };
}

export function measure(buffer) {
  const text = new TextDecoder('utf-8', { fatal: true }).decode(buffer).replace(/\r\n/g, '\n');
  const breaks = (text.match(/\n/g) ?? []).length;
  return { lines: breaks + (text.length > 0 && !text.endsWith('\n') ? 1 : 0), bytes: Buffer.byteLength(text, 'utf8') };
}

export function over(metrics, rule) {
  return metrics.lines > rule.maxLines || metrics.bytes > rule.maxBytes;
}
