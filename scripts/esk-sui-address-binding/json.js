const { fail } = require('./contract')

function parseUniqueJson(text) {
  if (typeof text !== 'string' || text.length === 0) fail('INVALID_INPUT')
  let cursor = 0

  function whitespace() {
    while (cursor < text.length && /[\u0020\u0009\u000a\u000d]/.test(text[cursor])) cursor += 1
  }

  function stringToken() {
    if (text[cursor] !== '"') fail('INVALID_INPUT')
    const start = cursor
    cursor += 1
    let escaped = false
    while (cursor < text.length) {
      const character = text[cursor]
      if (text.charCodeAt(cursor) < 0x20) fail('INVALID_INPUT')
      cursor += 1
      if (escaped) {
        escaped = false
      } else if (character === '\\') {
        escaped = true
      } else if (character === '"') {
        try { return JSON.parse(text.slice(start, cursor)) } catch { fail('INVALID_INPUT') }
      }
    }
    fail('INVALID_INPUT')
  }

  function numberToken() {
    const match = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/.exec(
      text.slice(cursor))
    if (!match) fail('INVALID_INPUT')
    cursor += match[0].length
  }

  function value() {
    whitespace()
    const character = text[cursor]
    if (character === '{') return objectToken()
    if (character === '[') return arrayToken()
    if (character === '"') { stringToken(); return }
    for (const literal of ['true', 'false', 'null']) {
      if (text.startsWith(literal, cursor)) { cursor += literal.length; return }
    }
    numberToken()
  }

  function objectToken() {
    cursor += 1
    whitespace()
    if (text[cursor] === '}') { cursor += 1; return }
    const keys = new Set()
    while (cursor < text.length) {
      whitespace()
      const key = stringToken()
      if (keys.has(key)) fail('INVALID_INPUT')
      keys.add(key)
      whitespace()
      if (text[cursor] !== ':') fail('INVALID_INPUT')
      cursor += 1
      value()
      whitespace()
      if (text[cursor] === '}') { cursor += 1; return }
      if (text[cursor] !== ',') fail('INVALID_INPUT')
      cursor += 1
    }
    fail('INVALID_INPUT')
  }

  function arrayToken() {
    cursor += 1
    whitespace()
    if (text[cursor] === ']') { cursor += 1; return }
    while (cursor < text.length) {
      value()
      whitespace()
      if (text[cursor] === ']') { cursor += 1; return }
      if (text[cursor] !== ',') fail('INVALID_INPUT')
      cursor += 1
    }
    fail('INVALID_INPUT')
  }

  value()
  whitespace()
  if (cursor !== text.length) fail('INVALID_INPUT')
  try { return JSON.parse(text) } catch { fail('INVALID_INPUT') }
}

module.exports = { parseUniqueJson }
