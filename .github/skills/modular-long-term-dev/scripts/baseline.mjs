import fs from 'node:fs';
import path from 'node:path';
import { BASELINE } from './policy.mjs';

export function writeBaseline(root, result, command, reason) {
  const destination = path.join(root, BASELINE);
  if (command === 'baseline') {
    if (fs.existsSync(destination)) throw new Error('Baseline already exists; use ratchet to decrease it');
    if (typeof reason !== 'string' || !reason.trim()) throw new Error('baseline requires --reason');
  } else {
    if (!result.baseline) throw new Error('No baseline to ratchet');
    if (result.failures.length) throw new Error('Resolve violations before ratcheting; new debt cannot be adopted');
  }
  const files = {};
  for (const row of result.rows.filter(item => item.debt)) {
    const prior = result.baseline?.files[row.path];
    files[row.path] = {
      lines: prior ? Math.min(prior.lines, row.lines) : row.lines,
      bytes: prior ? Math.min(prior.bytes, row.bytes) : row.bytes,
    };
  }
  const data = {
    version: 1, reason: result.baseline?.reason ?? reason.trim(),
    createdAt: result.baseline?.createdAt ?? new Date().toISOString(), files,
  };
  fs.writeFileSync(destination, JSON.stringify(data, null, 2) + '\n', {
    encoding: 'utf8', flag: command === 'baseline' ? 'wx' : 'w',
  });
  return Object.keys(files).length;
}
