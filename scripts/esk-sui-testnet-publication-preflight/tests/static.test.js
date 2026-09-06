'use strict'

const test = require('node:test')
const assert = require('node:assert/strict')
const { readFileSync, readdirSync } = require('node:fs')
const { join } = require('node:path')

const ROOT = join(__dirname, '..')
const ENTRY = join(ROOT, '../prepare-esk-sui-testnet-publication.js')

function runtimeSource() {
  const files = readdirSync(ROOT).filter(name => name.endsWith('.js')).sort()
  assert.ok(files.length >= 8, 'expected the modular preflight runtime')
  return [
    ...files.map(name => `// ${name}\n${readFileSync(join(ROOT, name), 'utf8')}`),
    `// prepare-esk-sui-testnet-publication.js\n${readFileSync(ENTRY, 'utf8')}`,
  ].join('\n')
}

test('runtime has no network, wallet, RPC, transaction, signing, broadcast, environment or process surface', () => {
  const source = runtimeSource()

  for (const forbidden of [
    /(?:require\s*\(|from\s+)["'](?:node:)?(?:http|https|http2|net|tls|dns|dgram)["']/,
    /(?:require\s*\(|from\s+)["'](?:node:)?child_process["']/,
    /(?:require\s*\(|from\s+)["']@mysten\//,
    /\bprocess\s*\.\s*env\b/,
    /\b(?:fetch|XMLHttpRequest|WebSocket)\s*\(/,
    /\b(?:SuiClient|SuiGrpcClient|SuiGraphQLClient|TransactionBlock|Transaction)\s*\(/,
    /\.\s*(?:signAndExecuteTransaction|signTransaction|signPersonalMessage|executeTransactionBlock|dryRunTransactionBlock|devInspectTransactionBlock)\s*\(/,
    /\b(?:spawn|spawnSync|exec|execSync|execFile|execFileSync|fork)\s*\(/,
    /\b(?:decodeSuiPrivateKey|fromSecretKey|getSecretKey)\s*\(/,
  ]) assert.doesNotMatch(source, forbidden)
})

test('runtime does not expose an implicit default candidate or user configuration path', () => {
  const source = runtimeSource()
  assert.doesNotMatch(source, /\.sui[\\/]sui_config|sui\.keystore|\.env(?:\W|$)/i)
  assert.doesNotMatch(source, /homedir\s*\(|userInfo\s*\(|APPDATA|USERPROFILE|HOME/)
})
