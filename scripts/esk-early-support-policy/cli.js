#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const { MAX_INPUT_BYTES, PolicyInputError, evaluatePolicyBuffer } = require('./contract');

function readBoundedFile(inputPath) {
  // Never open Windows device namespaces, reserved devices or network shares.
  const pieces = inputPath.replaceAll('\\', '/').split('/');
  if (/^[\\/]{2}/.test(inputPath)
      || pieces.some((part) => /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$/i.test(part))) {
    throw new PolicyInputError('INPUT_NOT_REGULAR_FILE');
  }
  let descriptor;
  try {
    // Check before opening so named pipes cannot wait for a writer. Repeat on
    // the descriptor to prevent a swapped path from bypassing the file check.
    if (!fs.lstatSync(inputPath).isFile()) throw new PolicyInputError('INPUT_NOT_REGULAR_FILE');
    descriptor = fs.openSync(inputPath,
      fs.constants.O_RDONLY | (fs.constants.O_NONBLOCK || 0) | (fs.constants.O_NOFOLLOW || 0));
    const before = fs.fstatSync(descriptor);
    if (!before.isFile()) throw new PolicyInputError('INPUT_NOT_REGULAR_FILE');
    if (before.size > MAX_INPUT_BYTES) throw new PolicyInputError('INPUT_TOO_LARGE');
    const buffer = Buffer.alloc(MAX_INPUT_BYTES + 1);
    let length = 0;
    while (length < buffer.length) {
      const count = fs.readSync(descriptor, buffer, length, buffer.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length > MAX_INPUT_BYTES) throw new PolicyInputError('INPUT_TOO_LARGE');
    return buffer.subarray(0, length);
  } catch (error) {
    if (error instanceof PolicyInputError) throw error;
    throw new PolicyInputError('INPUT_READ_FAILED');
  } finally {
    if (descriptor !== undefined) {
      try { fs.closeSync(descriptor); } catch { /* No path or OS message escapes. */ }
    }
  }
}

function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    return {
      schema: 'elon.esk.early_support_policy_cli_help.v1',
      usage: 'node scripts/esk-early-support-policy/cli.js --input <file>',
      max_input_bytes: MAX_INPUT_BYTES,
      offline: true,
    };
  }
  if (args.length !== 2 || args[0] !== '--input' || !args[1]
      || args[1].startsWith('--') || args[1] === '-') {
    throw new PolicyInputError('INVALID_ARGUMENTS');
  }
  return evaluatePolicyBuffer(readBoundedFile(args[1]));
}

try {
  process.stdout.write(`${JSON.stringify(main(process.argv.slice(2)))}\n`);
} catch (error) {
  const code = error instanceof PolicyInputError ? error.code : 'INTERNAL_ERROR';
  process.stdout.write(`${JSON.stringify({
    schema: 'elon.esk.early_support_policy_error.v1', error: { code },
  })}\n`);
  process.exitCode = 2;
}
