'use strict'

// This preload is used only by CLI tests. Any accidental network, process or
// Sui client import must fail before it can observe local configuration.
const Module = require('node:module')

const forbiddenModules = new Set([
  'http', 'node:http', 'https', 'node:https', 'http2', 'node:http2',
  'net', 'node:net', 'tls', 'node:tls', 'dns', 'node:dns',
  'dgram', 'node:dgram', 'child_process', 'node:child_process',
])
const load = Module._load
Module._load = function guardedLoad(request, parent, isMain) {
  if (forbiddenModules.has(request) || request.startsWith('@mysten/')) {
    throw new Error('OFFLINE_GUARD_BLOCKED')
  }
  return load.call(this, request, parent, isMain)
}

function blocked() { throw new Error('OFFLINE_GUARD_BLOCKED') }
Object.defineProperty(globalThis, 'fetch', { configurable: true, writable: true, value: blocked })
Object.defineProperty(globalThis, 'WebSocket', { configurable: true, writable: true, value: blocked })
