import { BASELINE, CONFIG, assertKeys, classify, measure, over, parseConfig } from './policy.mjs';
import { resolveRef, snapshot } from './git-snapshot.mjs';

export function parseBaseline(text) {
  if (text == null) return null;
  const data = JSON.parse(text);
  assertKeys(data, ['version', 'reason', 'createdAt', 'files'], BASELINE);
  if (data.version !== 1 || typeof data.reason !== 'string' || !data.reason.trim()) throw new Error('Invalid baseline metadata');
  assertKeys(data.files, Object.keys(data.files ?? {}), 'baseline files');
  for (const [file, metrics] of Object.entries(data.files)) {
    if (!file || file.startsWith('/') || file.includes('\\') || file.includes(':') || file.split('/').includes('..')) {
      throw new Error('Invalid baseline path ' + file);
    }
    assertKeys(metrics, ['lines', 'bytes'], 'baseline metrics');
    for (const key of ['lines', 'bytes']) {
      if (!Number.isSafeInteger(metrics[key]) || metrics[key] < 0) throw new Error('Invalid baseline metric ' + file);
    }
  }
  return data;
}

export function inspect(root, options = {}) {
  const current = snapshot(root, options.staged ? 'index' : 'worktree');
  const config = parseConfig(current.text(CONFIG));
  const baseline = parseBaseline(current.text(BASELINE));
  const previous = options.base ? snapshot(root, 'tree', resolveRef(root, options.base)) : null;
  const priorBaseline = parseBaseline(previous?.text(BASELINE));
  const failures = [];
  if (priorBaseline && baseline) {
    for (const [file, size] of Object.entries(baseline.files)) {
      const prior = priorBaseline.files[file];
      if (!prior || size.lines > prior.lines || size.bytes > prior.bytes) {
        failures.push({ path: file, problem: 'baseline-expanded', message: 'Existing baseline may only shrink' });
      }
    }
  }
  const files = [...current.entries.keys()].filter(file => classify(file, config)).sort();
  const contents = current.readMany(files);
  const oldContents = previous?.readMany(files.filter(file => previous.entries.has(file)));
  const rows = [];
  for (const file of files) {
    const rule = classify(file, config);
    let size;
    try { size = measure(contents.get(file)); }
    catch { throw new Error('Invalid UTF-8 source/document: ' + file); }
    const debt = baseline?.files[file];
    let allowedLines = Math.max(rule.maxLines, debt?.lines ?? 0);
    let allowedBytes = Math.max(rule.maxBytes, debt?.bytes ?? 0);
    if (priorBaseline) {
      const prior = priorBaseline.files[file];
      const old = oldContents?.has(file) ? measure(oldContents.get(file)) : null;
      // A removed/re-added or renamed giant is new debt, even if a stale entry remains.
      allowedLines = Math.min(allowedLines, Math.max(rule.maxLines, old && prior ? Math.min(old.lines, prior.lines) : 0));
      allowedBytes = Math.min(allowedBytes, Math.max(rule.maxBytes, old && prior ? Math.min(old.bytes, prior.bytes) : 0));
    }
    const row = { path: file, ...size, ...rule, allowedLines, allowedBytes, debt: over(size, rule) };
    rows.push(row);
    if (size.lines > allowedLines || size.bytes > allowedBytes) {
      failures.push({ ...row, problem: 'size-exceeded', message: 'Extract a focused module; do not expand the baseline' });
    }
  }
  return {
    mode: options.staged ? 'staged' : 'worktree', rows, failures, baseline,
    checked: rows.length, debt: rows.filter(row => row.debt).length,
    warnings: rows.filter(row => !row.debt && row.lines > 500),
  };
}

export function printable(result, verbose = false) {
  const lines = ['MODULARITY=' + (result.failures.length ? 'failed' : 'passed') +
    ' mode=' + result.mode + ' checked=' + result.checked + ' debt=' + result.debt +
    ' warnings=' + result.warnings.length];
  const rows = verbose ? result.rows.filter(row => row.debt || row.lines > 500) : result.failures;
  for (const row of rows) {
    lines.push(JSON.stringify(row.path) + ': ' + (row.problem ?? row.role) +
      (row.lines == null ? '' : ' lines=' + row.lines + '/' + row.allowedLines + ' bytes=' + row.bytes + '/' + row.allowedBytes));
  }
  return lines.join('\n');
}
