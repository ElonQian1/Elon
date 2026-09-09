'use strict';
const fs = require('node:fs'), path = require('node:path');
const assert = require('node:assert/strict'), test = require('node:test');
const ps = fs.readFileSync(path.join(__dirname, 'invoke-conversation-ui-acceptance.ps1'), 'utf8');
const java = fs.readFileSync(path.join(__dirname, 'android/ConversationUiAcceptance.java'), 'utf8');
const expressions = [
  ps.match(/\$productionModelSelector = '([^']+)'/)[1],
  JSON.parse('"' + java.match(/"(\^chatgpt-composer-option:model:[^"]+)"/)[1] + '"')
];
test('the selector crosses adb as base64 rather than shell words', () => {
  assert.match(ps, /\$parameters\.selector_b64 = \[Convert\]::ToBase64String\(\[Text.Encoding\]::UTF8.GetBytes\(\$Selector\)\)/);
  assert.doesNotMatch(ps, /\$parameters\.selector\s*=/);
  assert.match(java, /android\.util\.Base64\.decode\(\s*getParams\(\)\.getString\("selector_b64", ""\)/);
  for (const label of ['GPT-5.6 Sol', '\u6781\u9ad8']) {
    const value = 'chatgpt-composer-option:model:private_model_2_1:' + label;
    const encoded = Buffer.from(value).toString('base64');
    assert.match(encoded, /^[A-Za-z0-9+/=]+$/);
    assert.equal(Buffer.from(encoded, 'base64').toString('utf8'), value);
  }
});
for (const [index, expression] of expressions.entries()) {
  test('production model selectors remain exact and bounded in layer ' + index, () => {
    const pattern = new RegExp(expression);
    const prefix = 'chatgpt-composer-option:model:';
    for (const value of ['private_model_2_1:GPT-5.6 Sol', 'private_model_1_3:\u6781\u9ad8', 'model_1_0:High']) {
      assert.equal(pattern.test(prefix + value), true);
    }
    for (const value of ['id:', ':High', 'x'.repeat(97) + ':High', 'id:' + 'x'.repeat(121), 'id:High\nLow', 'id:High\rLow']) {
      assert.equal(pattern.test(prefix + value), false);
    }
    assert.equal(pattern.test('chatgpt-composer-option:tools:private_model_1_0:High'), false);
    assert.equal(pattern.test('chatgpt-conversation:private_model_1_0:High'), false);
  });
}
