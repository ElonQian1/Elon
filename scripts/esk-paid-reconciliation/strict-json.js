'use strict';

const { TextDecoder } = require('node:util');

const MAX_BYTES = 1048576;
const ERROR_CODES = new Set([
  'INVALID_INPUT', 'INVALID_JSON', 'DUPLICATE_JSON_KEY', 'INPUT_TOO_LARGE',
  'INVALID_UTF8', 'INPUT_TOO_DEEP', 'INVALID_AMOUNT', 'INPUT_TIMEOUT',
]);

class InputError extends Error {
  constructor(code = 'INVALID_INPUT') {
    const safeCode = ERROR_CODES.has(code) ? code : 'INVALID_INPUT';
    super(safeCode);
    this.name = 'InputError';
    this.code = safeCode;
  }
}

function fail(code = 'INVALID_JSON') {
  throw new InputError(code);
}

// Do not let JSON.parse erase duplicate keys or round numeric literals before
// validation. This contract uses numbers only for bounded integer fields.
function parseStrictJson(buffer) {
  if (!Buffer.isBuffer(buffer)) fail('INVALID_INPUT');
  if (buffer.length > MAX_BYTES) fail('INPUT_TOO_LARGE');
  if (buffer.length >= 3 && buffer[0] === 0xef && buffer[1] === 0xbb
      && buffer[2] === 0xbf) fail('INVALID_UTF8');
  let source;
  try {
    source = new TextDecoder('utf-8', { fatal: true }).decode(buffer);
  } catch {
    fail('INVALID_UTF8');
  }
  let offset = 0;
  const whitespace = () => {
    while (offset < source.length && /[ \t\r\n]/.test(source[offset])) offset += 1;
  };
  const string = () => {
    const start = offset;
    if (source[offset++] !== '"') fail();
    while (offset < source.length) {
      const character = source[offset++];
      if (character === '"') {
        try {
          return JSON.parse(source.slice(start, offset));
        } catch {
          fail();
        }
      }
      if (character.charCodeAt(0) < 0x20) fail();
      if (character !== '\\') continue;
      const escape = source[offset++];
      if (escape === 'u') {
        if (!/^[0-9a-fA-F]{4}$/.test(source.slice(offset, offset + 4))) fail();
        offset += 4;
      } else if (!escape || !'"\\/bfnrt'.includes(escape)) {
        fail();
      }
    }
    fail();
  };
  const value = (depth) => {
    whitespace();
    const character = source[offset];
    if (character === '"') return string();
    if (character === '{' || character === '[') {
      if (depth >= 12) fail('INPUT_TOO_DEEP');
      return character === '{' ? object(depth + 1) : array(depth + 1);
    }
    for (const [literal, decoded] of [['true', true], ['false', false], ['null', null]]) {
      if (source.startsWith(literal, offset)) {
        offset += literal.length;
        return decoded;
      }
    }
    const token = /^-?(?:0|[1-9][0-9]*)/.exec(source.slice(offset));
    if (!token) fail();
    offset += token[0].length;
    // Fractions/exponents, even those rounding to integers, are not accepted.
    if (/[.eE]/.test(source[offset] || '')) fail();
    const decoded = Number(token[0]);
    if (!Number.isSafeInteger(decoded) || Object.is(decoded, -0)) fail();
    return decoded;
  };
  const array = (depth) => {
    offset += 1;
    const result = [];
    whitespace();
    if (source[offset] === ']') {
      offset += 1;
      return result;
    }
    while (offset < source.length) {
      result.push(value(depth));
      whitespace();
      if (source[offset] === ']') {
        offset += 1;
        return result;
      }
      if (source[offset++] !== ',') fail();
    }
    fail();
  };
  const object = (depth) => {
    offset += 1;
    const result = {};
    const keys = new Set();
    whitespace();
    if (source[offset] === '}') {
      offset += 1;
      return result;
    }
    while (offset < source.length) {
      whitespace();
      const key = string();
      if (keys.has(key)) fail('DUPLICATE_JSON_KEY');
      keys.add(key);
      whitespace();
      if (source[offset++] !== ':') fail();
      const decoded = value(depth);
      Object.defineProperty(result, key, {
        value: decoded, enumerable: true, configurable: true, writable: true,
      });
      whitespace();
      if (source[offset] === '}') {
        offset += 1;
        return result;
      }
      if (source[offset++] !== ',') fail();
    }
    fail();
  };
  const result = value(0);
  whitespace();
  if (offset !== source.length) fail();
  return result;
}

module.exports = { MAX_BYTES, InputError, parseStrictJson };
