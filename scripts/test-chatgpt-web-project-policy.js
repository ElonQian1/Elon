'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_project_policy.js');
const hints = require('../android/app/src/main/assets/chatgpt_web_adapter_project_hints.js');

function node(label, attributes = {}, parent = null) {
  const children = [];
  const value = {
    textContent: label,
    parentElement: parent,
    children,
    getAttribute: (name) => attributes[name] || null,
    getAttributeNames: () => Object.keys(attributes),
    querySelectorAll: () => children,
    contains(candidate) {
      let current = candidate;
      while (current) {
        if (current === value) return true;
        current = current.parentElement;
      }
      return false;
    }
  };
  if (parent) parent.children.push(value);
  return value;
}

const root = node('');
const legacy = node('Legacy project', { href: '/g/g-p-legacy_1/project' }, root);
const modern = node('Modern project', { 'data-project-id': 'g-p-modern_2' }, root);
const react = node('React project', {}, root);
react.__reactProps$test = { project: { id: 'g-p-react_3' } };
const modernOptions = node('Open Modern project project options', {}, root);
const unresolved = node('Unresolved project', {}, root);
const unresolvedOptions = node('Open Unresolved project project options', {}, root);
const create = node('New project', {}, root);
const hidden = node('Private script metadata', { content: 'g-p-hidden_4' }, null);
const productionId = 'g-p-6916e3fec8d8819195edffedd2f6e08e';
const conversation = node('启动语音功能', { href: '/g/' + productionId + '/c/voice-chat' }, root);
conversation.__reactProps$test = { project: { id: productionId } };
const renamedProject = node('投资加密货币', {
  href: '/g/' + productionId + '-tou-zi-jia-mi-huo-bi/project'
}, root);
const documentMock = {
  querySelectorAll(selector) {
    if (selector === '*') return [
      root, legacy, modern, react, modernOptions, unresolved, unresolvedOptions, create, hidden,
      conversation, renamedProject
    ];
    if (selector === 'button, [role="button"]') return [modernOptions, unresolved, unresolvedOptions, create];
    return [
      legacy, modern, react, modernOptions, unresolved, unresolvedOptions, create,
      conversation, renamedProject
    ];
  }
};

assert.equal(policy.projectId('/g/g-p-demo_3/project'), 'g-p-demo_3');
assert.equal(policy.canonicalPath('g-p-demo_3'), '/g/g-p-demo_3/project');
assert.equal(policy.referencedTitle('Open Modern project project options'), 'Modern project');
assert.equal(policy.referencedTitle('打开“移动端项目”的项目选项'), '移动端项目');
assert.equal(policy.runtimeProjectId(react), 'g-p-react_3');
assert.equal(
  policy.projectId('/g/' + productionId + '-tou-zi-jia-mi-huo-bi/project'),
  productionId
);
const projects = policy.read(documentMock, () => true, (value) => value.textContent);
assert.deepEqual(
  projects.map((project) => project.id),
  ['g-p-legacy_1', 'g-p-modern_2', 'g-p-react_3', productionId]
);
assert.equal(projects.find((project) => project.id === productionId).title, '投资加密货币');
assert.equal(projects.some((project) => project.title === '启动语音功能'), false);
const mergedHints = hints.merge(
  [{ id: productionId, title: '投资加密货币', path: '/g/' + productionId + '-tou-zi-jia-mi-huo-bi/project' }],
  [{ id: productionId, title: '启动语音功能', path: '/g/' + productionId + '/project' }]
);
assert.equal(mergedHints.length, 1);
assert.equal(mergedHints[0].id, productionId);
assert.equal(mergedHints[0].title, '投资加密货币');
assert.equal(mergedHints[0].path, '/g/' + productionId + '/project');
assert.equal(policy.findNode(documentMock, '/g/g-p-modern_2/project', () => true, (value) => value.textContent), modern);
assert.equal(projects.some((project) => project.title === 'New project'), false);
assert.equal(projects.some((project) => project.id === 'g-p-hidden_4'), false);
assert.deepEqual(policy.unresolved(documentMock, () => true, (value) => value.textContent)
  .map((project) => project.title), ['Unresolved project']);
process.stdout.write('chatgpt project policy tests passed\n');
