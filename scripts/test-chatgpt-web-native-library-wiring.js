'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');

test('production UI routes folders and search to the native owner without dispatching conversation navigation', () => {
  const ui = fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/WebChatLibraryBrowser.kt'), 'utf8');
  const navigation = fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt'), 'utf8');
  assert.match(ui, /item\.kind == "directory"\) navigate\(item\.handle\)/);
  assert.match(ui, /requestLibraryFiles\(directory, query, operation\)/);
  assert.match(ui, /downloadLibraryFile\(file\.handle, file\.downloadHandle\)/);
  assert.match(ui, /WebChatFileDownloadDialog/);
  assert.match(ui, /setOnDismissListener/);
  assert.doesNotMatch(ui, /openConversation|loadUrl|evaluateJavascript|setDraft|startVoice/);
  assert.match(navigation, /feature\.kind == "library" && library\.show\(\)/);
});
