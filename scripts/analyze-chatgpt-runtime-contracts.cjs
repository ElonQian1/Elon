'use strict';

// Offline public-source evidence only. Candidate matches never enable a runtime profile.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const roles = { shared: '4813494d-', conversation: 'conversation-small-', composer: '8b34dbc2-' };
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
const parser = () => require(process.env.CHATGPT_AST_PARSER || 'acorn');

function walk(node, visit) {
  if (!node || typeof node !== 'object') return;
  if (node.type) visit(node);
  for (const [key, value] of Object.entries(node)) {
    if (key === 'start' || key === 'end') continue;
    if (Array.isArray(value)) value.forEach(child => walk(child, visit));
    else if (value && typeof value === 'object') walk(value, visit);
  }
}

function normalize(value, parent = {}, key = '') {
  if (typeof value === 'bigint') return { bigint: String(value) };
  if (Array.isArray(value)) return value.map(child => normalize(child, parent, key));
  if (!value || typeof value !== 'object') return value;
  if (value.type === 'Identifier') {
    const stable = (key === 'property' && !parent.computed) ||
      (key === 'key' && !parent.computed) || key === 'imported';
    return { type: value.type, name: stable ? value.name : '_' };
  }
  return Object.fromEntries(Object.entries(value)
    .filter(([name]) => !['start', 'end', 'raw'].includes(name))
    .map(([name, child]) => [name, normalize(child, value, name)]));
}

function fingerprint(node) { return hash(JSON.stringify(normalize(node))); }

function parseSource(text, acorn = parser()) {
  const ast = acorn.parse(text, { ecmaVersion: 'latest', sourceType: 'module' });
  const definitions = new Map(), exported = new Map(), imports = new Map(), sources = [];
  const topNames = new Set();
  for (const statement of ast.body) {
    if (statement.type === 'ImportDeclaration') {
      sources.push(statement.source.value);
      for (const spec of statement.specifiers) imports.set(spec.local.name,
        { file: statement.source.value, name: spec.imported?.name || spec.type });
    }
    if (statement.type === 'ExportNamedDeclaration') {
      for (const spec of statement.specifiers) exported.set(spec.exported.name, spec.local.name);
    }
    if (['FunctionDeclaration', 'ClassDeclaration'].includes(statement.type)) {
      definitions.set(statement.id.name, [statement]);
    }
    if (statement.type === 'VariableDeclaration') {
      for (const declaration of statement.declarations) {
        if (declaration.id.type !== 'Identifier') continue;
        topNames.add(declaration.id.name);
        if (declaration.init) definitions.set(declaration.id.name, [declaration.init]);
      }
    }
  }
  walk(ast, node => {
    if (node.type === 'AssignmentExpression' && node.operator === '=' &&
        node.left.type === 'Identifier' && topNames.has(node.left.name)) {
      const values = definitions.get(node.left.name) || [];
      values.push(node.right); definitions.set(node.left.name, values);
    }
  });
  const index = new Map();
  for (const [name, nodes] of definitions) if (nodes.length === 1) {
    const key = fingerprint(nodes[0]);
    index.set(key, [...(index.get(key) || []), name]);
  }
  return { text, definitions, exported, imports, sources, index };
}

function compareSymbol(old, current, name, localOnly = false) {
  const local = localOnly ? name : old.exported.get(name);
  const nodes = old.definitions.get(local);
  const candidates = nodes?.length === 1 ? current.index.get(fingerprint(nodes[0])) || [] : [];
  const found = candidates.map(candidate => ({ local: candidate,
    exports: [...current.exported].filter(([, value]) => value === candidate).map(([key]) => key) }));
  return { oldExport: localOnly ? null : name, oldLocal: local || null,
    definitions: nodes?.length || 0, candidates: found };
}

function roleFiles(sources) {
  const files = {};
  for (const [role, prefix] of Object.entries(roles)) {
    const matches = sources.filter(source => typeof source === 'string' &&
      new RegExp('^\\./' + prefix + '[A-Za-z0-9_-]+\\.js$').test(source));
    if (matches.length !== 1) throw Error('anchor_role_ambiguous:' + role);
    files[role] = matches[0].slice(2);
  }
  return files;
}

function readModule(directory, filename) {
  if (!/^[A-Za-z0-9][A-Za-z0-9_-]{0,95}\.js$/.test(filename)) throw Error('invalid_asset_filename');
  const bytes = fs.readFileSync(path.join(directory, filename));
  return { name: filename, sha256: hash(bytes), bytes: bytes.length,
    parsed: parseSource(bytes.toString('utf8')) };
}

function symbols(data) {
  return { exports: Object.fromEntries(data.exported), imports: Object.fromEntries(data.imports),
    definitions: Object.fromEntries([...data.definitions].map(([name, nodes]) =>
      [name, nodes.map(node => ({ start: node.start, end: node.end }))])) };
}

function analyze({ oldDir, newDir, prior, anchor }) {
  const root = readModule(newDir, anchor), files = roleFiles(root.parsed.sources);
  const report = { schema: 'elon.public_runtime_comparison.v1', advisory_only: true,
    anchor: { name: anchor, sha256: root.sha256, bytes: root.bytes }, files: {}, roles: {}, owners: {} };
  const indexes = {};
  for (const role of Object.keys(roles)) {
    const old = readModule(oldDir, prior.files[role]), current = readModule(newDir, files[role]);
    report.files[role] = { old: { name: old.name, sha256: old.sha256 },
      current: { name: current.name, sha256: current.sha256, bytes: current.bytes } };
    const map = { ...prior.expectedExports[role], ...prior.extraExports?.[role] };
    report.roles[role] = Object.fromEntries(Object.entries(map).map(([canonical, name]) =>
      [canonical, compareSymbol(old.parsed, current.parsed, name)]));
    indexes[role] = { old: symbols(old.parsed), current: symbols(current.parsed) };
    if (role === 'composer') {
      for (const [owner, name] of Object.entries({ tools: prior.toolOwner, temporary: prior.temporary?.owner })) {
        if (name) report.owners[owner] = compareSymbol(old.parsed, current.parsed, name, true);
      }
    }
  }
  return { report, indexes };
}

function main(argv) {
  if (argv.length !== 5) throw Error('usage: old-dir new-dir prior-fixture anchor-filename output-dir');
  const [oldDir, newDir, fixture, anchor, output] = argv;
  // Fixture is trusted repository code; downloaded modules are parsed, never imported/evaluated.
  const { report, indexes } = analyze({ oldDir, newDir, prior: require(path.resolve(fixture)), anchor });
  fs.mkdirSync(output, { recursive: true });
  fs.writeFileSync(path.join(output, 'comparison.json'), JSON.stringify(report, null, 2));
  for (const [role, data] of Object.entries(indexes)) {
    fs.writeFileSync(path.join(output, role + '-symbols.json'), JSON.stringify(data));
  }
  const entries = Object.entries(report.roles).flatMap(([role, items]) =>
    Object.entries(items).map(([name, result]) => ({ role, name, ...result })));
  console.log(JSON.stringify({ advisory_only: true, consumed: entries.length,
    unique: entries.filter(entry => entry.candidates.length === 1).length,
    review_required: entries.filter(entry => entry.candidates.length !== 1).map(entry => ({
      role: entry.role, name: entry.name, candidates: entry.candidates, definitions: entry.definitions })),
    owners: report.owners, files: report.files, anchor: report.anchor }));
}

module.exports = { normalize, fingerprint, parseSource, compareSymbol, roleFiles, analyze };
if (require.main === module) main(process.argv.slice(2));
