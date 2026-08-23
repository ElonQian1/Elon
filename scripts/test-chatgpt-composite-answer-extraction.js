'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../android/app/src/main/assets/chatgpt_web_adapter_messages.js'
), 'utf8');

class TextNode {
  constructor(value) {
    this.nodeType = 3;
    this.nodeValue = value;
  }
}

class ElementNode {
  constructor(text = '', options = {}) {
    this.nodeType = 1;
    this.tagName = options.tagName || 'DIV';
    this.attributes = options.attributes || {};
    this.childNodes = text ? [new TextNode(text)] : [];
    this.children = [];
    this.parentElement = options.parent || null;
    this.candidates = options.candidates || [];
    this.visible = options.visible !== false;
    this.actionContainer = Boolean(options.actionContainer);
    this.roleChild = options.roleChild || null;
    this.speakerHeading = options.speakerHeading || null;
    this.id = options.id || '';
  }

  get textContent() {
    return this.childNodes.map((child) => child.nodeValue || child.textContent || '').join('');
  }

  get innerText() { return this.textContent; }
  getAttribute(name) { return this.attributes[name] || null; }
  matches(selector) {
    if (selector === '[data-message-author-role]') return Boolean(this.attributes['data-message-author-role']);
    if (selector === '[role="listitem"]') return this.attributes.role === 'listitem';
    return false;
  }
  querySelector(selector) {
    if (selector === '[data-message-author-role]') return this.roleChild;
    return null;
  }
  querySelectorAll(selector) {
    if (selector.startsWith(':scope > h1')) return this.speakerHeading ? [this.speakerHeading] : [];
    if (selector.includes('.markdown') || selector.includes('[data-message-content]')) return this.candidates;
    return [];
  }
  getBoundingClientRect() { return this.visible ? { width: 400, height: 40 } : { width: 0, height: 0 }; }
  closest() { return this.actionContainer ? this : null; }
  contains(other) {
    for (let current = other && other.parentElement; current; current = current.parentElement) {
      if (current === this) return true;
    }
    return false;
  }
}

const window = {
  getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
};
const context = {
  window,
  location: { origin: 'https://chatgpt.com' },
  document: {},
  Node: { TEXT_NODE: 3, ELEMENT_NODE: 1 },
  Element: ElementNode,
  URL,
  Set,
  Array,
  String,
  Number,
  Math,
  Object,
};
vm.runInNewContext(source, context, { filename: 'chatgpt_web_adapter_messages.js' });
const messages = window.__elonChatGptMessages;

assert.equal(messages.isAssistantActionText('提供反馈'), true);
assert.equal(messages.isAssistantActionText('复制 分享 重新生成'), true);
assert.equal(messages.isAssistantActionText('提供反馈，但正文必须保留。'), false);

const feedback = new ElementNode('提供反馈');
const prose = new ElementNode('以太坊当前走势震荡，以下是完整分析。');
const owner = new ElementNode('', {
  attributes: { 'data-message-author-role': 'assistant' },
  candidates: [feedback, prose],
});
assert.deepEqual(
  messages.contentNodes(owner, 'assistant'),
  [feedback, prose],
  'sibling content islands stay in DOM order',
);
assert.equal(
  messages.messageContent(owner, 'assistant'),
  '以太坊当前走势震荡，以下是完整分析。',
  'action-only feedback chrome must not become the assistant answer',
);

const wrapper = new ElementNode('wrapper');
const nested = new ElementNode('唯一正文', { parent: wrapper });
owner.candidates = [wrapper, nested];
assert.deepEqual(
  messages.contentNodes(owner, 'assistant'),
  [nested],
  'nested markdown containers are serialized once',
);

const toolbar = new ElementNode('复制', { actionContainer: true });
owner.candidates = [toolbar, prose];
assert.deepEqual(messages.contentNodes(owner, 'assistant'), [prose]);

const roleIsland = new ElementNode('', {
  attributes: { 'data-message-author-role': 'assistant' },
  candidates: [feedback],
});
const turn = new ElementNode('', {
  attributes: { 'data-testid': 'conversation-turn-2' },
  candidates: [feedback, prose],
  roleChild: roleIsland,
});
assert.equal(messages.messageScope(turn), turn);
assert.equal(
  messages.messageContent(turn, 'assistant'),
  '以太坊当前走势震荡，以下是完整分析。',
  'the whole conversation turn must include prose rendered beside the nested role island',
);

const guestUserBody = new ElementNode('请展示苹果公司 AAPL 最近行情');
const guestAssistantBody = new ElementNode('苹果公司 AAPL：开盘 312.05，最高 312.38，最低 307.01，收盘 309.35。');
const guestUserTurn = new ElementNode('', {
  attributes: { role: 'listitem' },
  id: '884476ce-d131-403d-8dff-9d9942e54f41',
  speakerHeading: new ElementNode('你说：', { tagName: 'H5' }),
  candidates: [guestUserBody],
});
const guestAssistantTurn = new ElementNode('', {
  attributes: { role: 'listitem' },
  id: 'd725c535-6d33-4477-8981-8ecc0a624ba2',
  speakerHeading: new ElementNode('ChatGPT 说：', { tagName: 'H5' }),
  candidates: [guestAssistantBody],
});
const guestMain = {
  querySelectorAll(selector) {
    if (selector === '[role="listitem"], li, article') return [guestUserTurn, guestAssistantTurn];
    return [];
  },
};
context.document.querySelector = (selector) => selector === 'main' ? guestMain : null;
assert.deepEqual(
  messages.readMessages(false).map((message) => ({
    id: message.id,
    role: message.role,
    text: message.content[0].text,
  })),
  [
    {
      id: '884476ce-d131-403d-8dff-9d9942e54f41',
      role: 'user',
      text: '请展示苹果公司 AAPL 最近行情',
    },
    {
      id: 'd725c535-6d33-4477-8981-8ecc0a624ba2',
      role: 'assistant',
      text: '苹果公司 AAPL：开盘 312.05，最高 312.38，最低 307.01，收盘 309.35。',
    },
  ],
  'guest message-list turns are recognized from their accessible speaker headings',
);

console.log('CHATGPT_COMPOSITE_ANSWER_EXTRACTION=passed');
