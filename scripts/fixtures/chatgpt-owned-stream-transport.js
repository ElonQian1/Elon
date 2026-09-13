'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

// Compose production projection/ownership modules; only the network is fake.
function install(page) {
  const url = new URL(page.location.href);
  Object.assign(page.location, { origin: url.origin, pathname: url.pathname });
  page.__elonChatGptPrivateStreamObserverEnabled = true;
  page.fetch ||= async () => { throw Error('unexpected_passive_fetch'); };
  const sandbox = { window: page, location: page.location, URL, Promise, Date, JSON,
    TextDecoder, DecompressionStream, Blob, Uint8Array, atob, Set, Object, String, Number, Array, RegExp };
  for (const name of ['delta_document', 'stream_policy', 'owned_stream', 'stream_transport']) {
    const filename = 'chatgpt_web_private_' + name + '.js';
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../../android/app/src/main/assets', filename), 'utf8'),
      sandbox, { filename });
  }
  return page.__elonChatGptPrivateStreamTransport;
}

module.exports = { install };
