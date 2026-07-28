const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const repoRoot = path.resolve(__dirname, '..');
const pagePath = path.join(repoRoot, 'server', 'src', 'assets', 'web_page.html');
const source = fs.readFileSync(pagePath, 'utf8');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function stripTemplatePlaceholders(value) {
  return value.replace(/__UI_TUNER_[A-Z0-9_]+__/g, '');
}

function assertBalancedBraces(value, label) {
  let depth = 0;
  let quote = '';
  let escaped = false;
  let blockComment = false;

  for (let index = 0; index < value.length; index += 1) {
    const current = value[index];
    const next = value[index + 1];
    if (blockComment) {
      if (current === '*' && next === '/') {
        blockComment = false;
        index += 1;
      }
      continue;
    }
    if (quote) {
      if (escaped) escaped = false;
      else if (current === '\\') escaped = true;
      else if (current === quote) quote = '';
      continue;
    }
    if (current === '/' && next === '*') {
      blockComment = true;
      index += 1;
    } else if (current === '"' || current === "'") {
      quote = current;
    } else if (current === '{') {
      depth += 1;
    } else if (current === '}') {
      depth -= 1;
      assert(depth >= 0, `${label} has an unexpected closing brace`);
    }
  }
  assert(!blockComment, `${label} has an unclosed block comment`);
  assert(!quote, `${label} has an unclosed quoted value`);
  assert(depth === 0, `${label} has unbalanced braces`);
}

assert(source.includes('<!DOCTYPE html>') || source.includes('<!doctype html>'), 'mobile PWA must keep a doctype');
assert(source.includes('<meta name="viewport"'), 'mobile PWA must keep its viewport metadata');

const styleBlocks = [...source.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)];
assert(styleBlocks.length > 0, 'mobile PWA must contain an inline style block');
styleBlocks.forEach((match, index) => assertBalancedBraces(match[1], `style block ${index + 1}`));

let checkedScripts = 0;
for (const match of source.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
  const attributes = match[1];
  if (/\bsrc\s*=/.test(attributes)) continue;
  const script = stripTemplatePlaceholders(match[2]).trim();
  if (!script) continue;
  new vm.Script(script, { filename: `${pagePath}:inline-script-${checkedScripts + 1}` });
  checkedScripts += 1;
}
assert(checkedScripts > 0, 'mobile PWA must contain a checkable inline script');

console.log(`MOBILE_PWA_SOURCE=passed styles=${styleBlocks.length} scripts=${checkedScripts}`);

