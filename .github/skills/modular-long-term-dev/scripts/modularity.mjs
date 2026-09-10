#!/usr/bin/env node
import { inspect, printable } from './check.mjs';
import { repository } from './git-snapshot.mjs';
import { writeBaseline } from './baseline.mjs';

const usage = 'modularity.mjs <check|audit|baseline|ratchet> [--root repo] [--staged] [--base ref] [--json] [--reason text]';

function main(args) {
  if (!args.length || args.includes('--help')) { console.log(usage); return 0; }
  const command = args.shift();
  if (!['check', 'audit', 'baseline', 'ratchet'].includes(command)) throw new Error(usage);
  const options = { root: process.cwd() };
  while (args.length) {
    const option = args.shift();
    if (['--staged', '--json'].includes(option)) options[option.slice(2)] = true;
    else if (['--root', '--base', '--reason'].includes(option)) {
      const value = args.shift();
      if (!value || value.startsWith('--')) throw new Error('Missing value for ' + option);
      options[option.slice(2)] = value;
    } else throw new Error('Unknown option ' + option);
  }
  if (['baseline', 'ratchet'].includes(command) && (options.staged || options.base)) {
    throw new Error('Baseline mutations require the current worktree, without --base or --staged');
  }
  const root = repository(options.root);
  const result = inspect(root, options);
  if (command === 'baseline' || command === 'ratchet') {
    const count = writeBaseline(root, result, command, options.reason);
    console.log('MODULARITY_BASELINE=' + command + ' files=' + count);
    return 0;
  }
  if (options.json) console.log(JSON.stringify(result, null, 2));
  else console.log(printable(result, command === 'audit'));
  return command === 'audit' ? 0 : result.failures.length ? 1 : 0;
}

try { process.exitCode = main(process.argv.slice(2)); }
catch (error) { console.error('MODULARITY_ERROR=' + error.message); process.exitCode = 2; }
