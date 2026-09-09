'use strict';
const path = require('node:path');
const assets = path.join(__dirname, '../../android/app/src/main/assets');
const moduleApi = require(path.join(assets, 'chatgpt_web_private_directory_refresh.js'));
const pages = require(path.join(assets, 'chatgpt_web_private_directory_pages.js'));
const jsonRequest = require(path.join(assets, 'chatgpt_web_private_json_request.js'));
const flush = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };
const response = (payload, status = 200) => ({ ok: status === 200, status, text: async () => JSON.stringify(payload) });
const emptyPage = url => url.includes('/snorlax/') ? { items: [], cursor: null } :
  { items: [], offset: 0, limit: 28, total: 0 };
function fixture(fetch = async url => response(emptyPage(url))) {
  const calls = [], accepted = [];
  const headers = { Authorization: 'Bearer synthetic-directory-auth', 'chatgpt-account-id': 'fixture-account' };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/g/g-p-fixture/project',
    pathname: '/g/g-p-fixture/project' },
    __elonChatGptDocumentToken: 'doc_directory_fixture', __elonChatGptPrivateJsonRequest: jsonRequest,
    __elonChatGptPrivateDirectoryRefresh: moduleApi,
    __elonChatGptPrivateDirectoryPages: pages, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTransport: {
      copySameOriginRequestHeaders: () => ({ ...headers }),
      acquireSameOriginRequestHeaders: async () => ({ ...headers }),
    },
  };
  const controller = moduleApi.create(root, (...args) => { accepted.push(args); return true; },
    (url, init) => { calls.push({ url, init }); return fetch(url, init); });
  return { root, calls, accepted, headers, controller };
}
module.exports = { assets, fixture, response, emptyPage, deferred, flush, pages };
