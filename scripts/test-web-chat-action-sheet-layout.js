'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

test('updatable action list retains a shrinkable vertical layout weight', () => {
  const source = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/WebChatActionSheet.kt'), 'utf8');
  const render = source.slice(source.indexOf('fun renderItems('), source.indexOf('val root ='));
  assert.match(render, /itemScroll\.layoutParams = LinearLayout\.LayoutParams\(\s*LinearLayout\.LayoutParams\.MATCH_PARENT,\s*dp\(activity, \(updatedItems\.size \* ITEM_HEIGHT_DP\)\.coerceAtMost\(MAX_LIST_HEIGHT_DP\)\),\s*(?:\/\/[^\n]*\n\s*)?1f,/);
  assert.match(source, /return WebChatActionSheetHandle\(dialog\).*\{[\s\S]*itemScroll\.post \{ renderItems\(updatedItems\) \}/);
});

test('native sharing acceptance reveals its action before clicking', () => {
  const source = fs.readFileSync(path.join(__dirname, 'android/ConversationUiAcceptance.java'), 'utf8');
  const step = source.split('case "conversation_actions":')[1].split('break;')[0];
  assert.match(step, /revealConversationAction\("web-chat-conversation-action-share"\)/);
});
