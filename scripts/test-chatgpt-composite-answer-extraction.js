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
    this.id = '';
  }

  get textContent() {
    return this.childNodes.map((child) => child.nodeValue || child.textContent || '').join('');
  }

  get innerText() { return this.textContent; }
  getAttribute(name) { return this.attributes[name] || null; }
  matches(selector) { return selector === '[data-message-author-role]' && Boolean(this.attributes['data-message-author-role']); }
  querySelector(selector) {
    if (selector === '[data-message-author-role]') return null;
    return null;
  }
  querySelectorAll(selector) {
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

console.log('CHATGPT_COMPOSITE_ANSWER_EXTRACTION=passed');
