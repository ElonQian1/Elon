#!/usr/bin/env node
'use strict';

const { readStandardInput } = require('../preview-esk-paid-reconciliation');
const { MAX_INPUT_BYTES, LedgerInputError, evaluateLedgerBuffer, failureReport } = require('./preview');

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    process.stdout.write(`${JSON.stringify({
      schema: 'elon.esk.economic_ledger_preview_cli_help.v1',
      usage: 'node scripts/esk-economic-ledger/cli.js < input.json',
      max_input_bytes: MAX_INPUT_BYTES, offline: true,
    })}\n`);
    return;
  }
  let report;
  try {
    if (args.length || process.stdin.isTTY) throw new LedgerInputError('INVALID_ARGUMENTS');
    report = evaluateLedgerBuffer(await readStandardInput(process.stdin));
  } catch (error) { report = failureReport(error); }
  process.stdout.write(`${JSON.stringify(report)}\n`);
  process.exitCode = report.review_status === 'consistent' ? 0 : 2;
}

if (require.main === module) main(process.argv.slice(2)).catch(() => {
  process.stdout.write(`${JSON.stringify(failureReport(new LedgerInputError('INTERNAL_ERROR')))}\n`);
  process.exitCode = 2;
});

module.exports = { main };
