/** Browser-compatible JSON decoding for credential and private-asset responses. */
const MAX_DEPTH = 32;
const MAX_BYTES = 1048576;
const unsafeKeys = new Set(['__proto__', 'constructor', 'prototype']);

function invalid() {
  const error = new Error('invalid_response');
  error.code = 'invalid_response';
  throw error;
}

export function decodeStrictJson(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.byteLength > MAX_BYTES ||
      (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf)) invalid();
  let source;
  try { source = new TextDecoder('utf-8', { fatal: true }).decode(bytes); }
  catch { invalid(); }
  let offset = 0;
  const numeric = /-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/y;
  const whitespace = () => {
    while (offset < source.length && /[ \t\r\n]/u.test(source[offset])) offset += 1;
  };
  const string = () => {
    const start = offset;
    if (source[offset++] !== '"') invalid();
    while (offset < source.length) {
      const character = source[offset++];
      if (character === '"') {
        let decoded;
        try { decoded = JSON.parse(source.slice(start, offset)); } catch { invalid(); }
        for (const scalar of decoded) {
          const point = scalar.codePointAt(0);
          if (point >= 0xd800 && point <= 0xdfff) invalid();
        }
        return decoded;
      }
      if (character.charCodeAt(0) < 0x20) invalid();
      if (character !== '\\') continue;
      const escape = source[offset++];
      if (escape === 'u') {
        if (!/^[0-9a-fA-F]{4}$/u.test(source.slice(offset, offset + 4))) invalid();
        offset += 4;
      } else if (!escape || !'"\\/bfnrt'.includes(escape)) invalid();
    }
    invalid();
  };
  const value = depth => {
    whitespace();
    const first = source[offset];
    if (first === '"') return string();
    if (first === '{' || first === '[') {
      if (depth >= MAX_DEPTH) invalid();
      return first === '{' ? object(depth + 1) : array(depth + 1);
    }
    for (const [literal, decoded] of [['true', true], ['false', false], ['null', null]]) {
      if (source.startsWith(literal, offset)) { offset += literal.length; return decoded; }
    }
    numeric.lastIndex = offset;
    const match = numeric.exec(source);
    if (!match) invalid();
    offset = numeric.lastIndex;
    const number = Number(match[0]);
    if (!Number.isFinite(number)) invalid();
    return number;
  };
  const array = depth => {
    offset += 1;
    const result = [];
    whitespace();
    if (source[offset] === ']') { offset += 1; return result; }
    while (offset < source.length) {
      result.push(value(depth));
      whitespace();
      if (source[offset] === ']') { offset += 1; return result; }
      if (source[offset++] !== ',') invalid();
    }
    invalid();
  };
  const object = depth => {
    offset += 1;
    const result = {};
    const keys = new Set();
    whitespace();
    if (source[offset] === '}') { offset += 1; return result; }
    while (offset < source.length) {
      whitespace();
      const key = string();
      if (unsafeKeys.has(key) || keys.has(key)) invalid();
      keys.add(key);
      whitespace();
      if (source[offset++] !== ':') invalid();
      Object.defineProperty(result, key, {
        value: value(depth), enumerable: true, configurable: true, writable: true,
      });
      whitespace();
      if (source[offset] === '}') { offset += 1; return result; }
      if (source[offset++] !== ',') invalid();
    }
    invalid();
  };
  const result = value(0);
  whitespace();
  if (offset !== source.length) invalid();
  return result;
}
