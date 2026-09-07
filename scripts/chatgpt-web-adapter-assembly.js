'use strict';
const fs = require('node:fs');
const path = require('node:path');
const root = path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/chatgptweb');

// Read both real production owners: ordered assets and WebView wiring.
function readAdapterSource() {
  const adapter = fs.readFileSync(path.join(root, 'ChatGptWebPageAdapter.kt'), 'utf8');
  if (!adapter.includes('private val ADAPTER_ASSETS = ChatGptWebAdapterAssets.names')) {
    throw new Error('Page adapter must use the production asset catalog');
  }
  return fs.readFileSync(path.join(root, 'ChatGptWebAdapterAssets.kt'), 'utf8') + '\n' + adapter;
}

module.exports = { readAdapterSource };
