'use strict';

// Public source inspection only: never executes the manifest or website modules.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const acorn = require('../pc-frontend/node_modules/acorn');
const [directory, mode = 'index', query = ''] = process.argv.slice(2);
if (!directory || !path.isAbsolute(directory)) throw Error('absolute evidence directory required');
const parse = source => acorn.parse(source, { ecmaVersion: 'latest', sourceType: 'module' });
const key = node => node?.name ?? node?.value;
const property = (node, name) => node.properties.find(item => key(item.key) === name)?.value;
const walk = (node, visit) => {
  if (!node || typeof node !== 'object') return;
  if (node.type) visit(node);
  for (const [name, value] of Object.entries(node)) {
    if (name === 'start' || name === 'end') continue;
    if (Array.isArray(value)) value.forEach(item => walk(item, visit));
    else if (value && typeof value === 'object') walk(value, visit);
  }
};

async function download() {
  const manifestName = query || 'manifest-492fbfe6.js';
  if (!/^manifest-[a-f0-9]+\.js$/.test(manifestName)) throw Error('observed manifest basename required');
  const manifest = fs.readFileSync(path.join(directory, manifestName), 'utf8');
  const root = parse(manifest).body[0].expression.right;
  const routes = property(root, 'routes');
  const entries = [property(root, 'entry'), ...['root', 'home.route', 'conversation-layout.route',
    'conversation.route'].map(name => property(routes, name))];
  const files = new Set(entries.flatMap(entry => [property(entry, 'module').value,
    ...property(entry, 'imports').elements.map(item => item.value)]));
  for (const file of files) {
    if (!/^\/cdn\/assets\/\d+\.[a-f0-9]+\.js$/.test(file)) throw Error('unexpected public asset path');
    const output = path.join(directory, path.basename(file));
    if (fs.existsSync(output)) continue;
    const bytes = execFileSync(process.platform === 'win32' ? 'curl.exe' : 'curl',
      ['--fail', '--silent', '--show-error', '--max-time', '30', '--proto', '=https',
        '--max-filesize', String(12 * 1024 * 1024), 'https://chatgpt.com' + file],
      { maxBuffer: 12 * 1024 * 1024, windowsHide: true });
    if (bytes.length > 12 * 1024 * 1024) throw Error('public asset size limit');
    fs.writeFileSync(output, bytes);
  }
  console.log(JSON.stringify({ publicAssets: files.size, directory }));
}

function inspect() {
  const [selection, needle = ''] = query.split('@');
  const found = [];
  const parseGaps = [];
  let modules = 0;
  for (const file of fs.readdirSync(directory).filter(name => /^\d+\.[a-f0-9]+\.js$/.test(name))) {
    const bytes = fs.readFileSync(path.join(directory, file));
    const source = bytes.toString('utf8');
    let ast;
    try { ast = parse(source); } catch (error) {
      // Credential-filtered research exports may no longer be valid JavaScript.
      parseGaps.push({ file, error: error.message });
      continue;
    }
    for (const statement of ast.body) {
      const declarations = statement.declaration?.declarations || [];
      const table = declarations.find(item => item.id.name === '__webpack_modules__')?.init;
      if (!table || table.type !== 'ObjectExpression') continue;
      for (const item of table.properties) {
        modules++;
        const body = source.slice(item.value.start, item.value.end);
        if (selection && (selection.startsWith('module:') ? key(item.key) !== selection.slice(7) : !body.includes(selection))) continue;
        const offset = needle ? Math.max(0, body.indexOf(needle) - 100) : 0;
        const exports = [];
        const classes = [];
        walk(item.value.body, node => {
          if (node.type === 'CallExpression' && node.callee.type === 'MemberExpression' &&
              key(node.callee.property) === 'd' && node.arguments[1]?.type === 'ObjectExpression') {
            for (const entry of node.arguments[1].properties) exports.push(key(entry.key));
          }
          if (node.type === 'ClassDeclaration' || node.type === 'ClassExpression') {
            classes.push({ name: node.id?.name, methods: node.body.body.map(member => key(member.key)) });
          }
        });
        found.push({ file, module: key(item.key), bytes: body.length, exports: exports.slice(0, 50),
          moduleSha256: crypto.createHash('sha256').update(body).digest('hex'),
          ...(mode === 'source' ? { source: body.slice(offset, offset + 16000) } : {}),
          ...(mode === 'snippets' ? { snippets: [...body.matchAll(new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'))]
            .slice(0, 4).map(match => body.slice(Math.max(0, match.index - 450), match.index + query.length + 900)) } : {}),
          ...(mode === 'methods' ? { classes } : {}),
          sha256: crypto.createHash('sha256').update(bytes).digest('hex') });
      }
    }
  }
  console.log(JSON.stringify({ modules, parseGaps, matched: found.length, items: found.slice(0, mode === 'source' ? 3 : 30) }, null, 2));
}

function prior() {
  const fixture = require('./fixtures/chatgpt-runtime-bindings-sep22b.cjs');
  const [role, wanted = ''] = query.split(':');
  if (!fixture.files[role]) throw Error('unknown prior role');
  const source = fs.readFileSync(path.join(directory, fixture.files[role]), 'utf8');
  const ast = parse(source), declarations = new Map(), aliases = new Map();
  walk(ast, node => {
    if (node.type === 'ExportSpecifier') aliases.set(key(node.exported), key(node.local));
    if (node.type === 'VariableDeclarator' && node.id.type === 'Identifier') declarations.set(node.id.name, node);
    if (node.type === 'AssignmentExpression' && node.left.type === 'Identifier') declarations.set(node.left.name, node);
    if (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') declarations.set(node.id.name, node);
  });
  for (const name of wanted.split(',')) {
    const symbol = aliases.get(fixture.expectedExports[role]?.[name]);
    const node = declarations.get(symbol);
    console.log(JSON.stringify({ name, symbol, source: node && source.slice(node.start, node.end).slice(0, 1800) }));
  }
}

(mode === 'download' ? download() : Promise.resolve().then(mode === 'prior' ? prior : inspect))
  .catch(error => { console.error(error.message); process.exitCode = 1; });
