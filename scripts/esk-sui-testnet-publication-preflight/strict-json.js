'use strict'

const { TextDecoder } = require('node:util')
const { fail } = require('./contract')

function parseStrictJson(buffer, maximumBytes) {
  if (!Buffer.isBuffer(buffer)) fail('INVALID_JSON')
  if (buffer.length > maximumBytes) fail('INPUT_TOO_LARGE')
  if (buffer.length >= 3 && buffer[0] === 0xef && buffer[1] === 0xbb && buffer[2] === 0xbf) {
    fail('INVALID_UTF8')
  }
  let source
  try { source = new TextDecoder('utf-8', { fatal: true }).decode(buffer) } catch {
    fail('INVALID_UTF8')
  }
  let offset = 0

  const whitespace = () => {
    while (offset < source.length && /[ \t\r\n]/.test(source[offset])) offset += 1
  }
  const stringToken = () => {
    const start = offset
    if (source[offset++] !== '"') fail('INVALID_JSON')
    while (offset < source.length) {
      const character = source[offset++]
      if (character === '"') {
        let decoded
        try { decoded = JSON.parse(source.slice(start, offset)) } catch { fail('INVALID_JSON') }
        for (let index = 0; index < decoded.length; index += 1) {
          const code = decoded.charCodeAt(index)
          if (code >= 0xd800 && code <= 0xdbff) {
            const next = decoded.charCodeAt(index + 1)
            if (!(next >= 0xdc00 && next <= 0xdfff)) fail('INVALID_JSON')
            index += 1
          } else if (code >= 0xdc00 && code <= 0xdfff) {
            fail('INVALID_JSON')
          }
        }
        return decoded
      }
      if (character.charCodeAt(0) < 0x20) fail('INVALID_JSON')
      if (character !== '\\') continue
      const escape = source[offset++]
      if (escape === 'u') {
        if (!/^[0-9a-fA-F]{4}$/.test(source.slice(offset, offset + 4))) fail('INVALID_JSON')
        offset += 4
      } else if (!escape || !'"\\/bfnrt'.includes(escape)) {
        fail('INVALID_JSON')
      }
    }
    fail('INVALID_JSON')
  }
  const value = (depth) => {
    whitespace()
    const character = source[offset]
    if (character === '"') return stringToken()
    if (character === '{' || character === '[') {
      if (depth >= 16) fail('INVALID_JSON')
      return character === '{' ? objectToken(depth + 1) : arrayToken(depth + 1)
    }
    for (const [literal, decoded] of [['true', true], ['false', false], ['null', null]]) {
      if (source.startsWith(literal, offset)) {
        offset += literal.length
        return decoded
      }
    }
    const token = source.slice(offset).match(/^-?(?:0|[1-9][0-9]*)/)
    if (!token) fail('INVALID_JSON')
    offset += token[0].length
    if (/[.eE]/.test(source[offset] || '')) fail('INVALID_JSON')
    const decoded = Number(token[0])
    if (!Number.isSafeInteger(decoded) || Object.is(decoded, -0)) fail('INVALID_JSON')
    return decoded
  }
  const arrayToken = (depth) => {
    offset += 1
    const result = []
    whitespace()
    if (source[offset] === ']') { offset += 1; return result }
    while (offset < source.length) {
      result.push(value(depth))
      whitespace()
      if (source[offset] === ']') { offset += 1; return result }
      if (source[offset++] !== ',') fail('INVALID_JSON')
    }
    fail('INVALID_JSON')
  }
  const objectToken = (depth) => {
    offset += 1
    const result = Object.create(null)
    const keys = new Set()
    whitespace()
    if (source[offset] === '}') { offset += 1; return result }
    while (offset < source.length) {
      whitespace()
      const key = stringToken()
      if (keys.has(key)) fail('DUPLICATE_JSON_KEY')
      keys.add(key)
      whitespace()
      if (source[offset++] !== ':') fail('INVALID_JSON')
      Object.defineProperty(result, key, {
        value: value(depth), enumerable: true, configurable: true, writable: true,
      })
      whitespace()
      if (source[offset] === '}') { offset += 1; return result }
      if (source[offset++] !== ',') fail('INVALID_JSON')
    }
    fail('INVALID_JSON')
  }

  const result = value(0)
  whitespace()
  if (offset !== source.length) fail('INVALID_JSON')
  return result
}

module.exports = { parseStrictJson }
