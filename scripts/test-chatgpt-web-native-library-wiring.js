'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');

test('production composer displays official staged attachments and reuses canonical removal', () => {
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  const strip = read('WebChatComposerAttachmentStrip.kt');
  assert.match(read('MainInputComposerSetup.kt'), /WebChatComposerAttachmentStrip\(pendingAttachmentHost\)/);
  assert.match(read('MainSocialAiChatFeature.kt'), /webAttachmentStrip\.render\(controller\.consumerPort\(\), consumerState\)/);
  assert.match(read('WebChatComposerProviderPresentation.kt'), /webAttachmentStrip\.hide\(\)/);
  assert.match(read('chatgptweb/ChatGptWebConsumerPortAdapter.kt'), /current\?\.attachments\.orEmpty\(\)\.map/);
  assert.match(strip, /port\.removeComposerAttachment\(item\.id\)/);
  assert.match(strip, /state\.pageUrl != expectedPage/);
  assert.match(strip, /state\.attachments\.none \{ it\.id == item\.id \}/);
  assert.doesNotMatch(strip, /evaluateJavascript|loadUrl|setDraft|sendText|uploadFile|requestLibraryFiles/);
});

test('native attach waits for the exact receipt and returns without navigating or sending text', () => {
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  const ui = read('WebChatLibraryAttachmentAction.kt');
  assert.match(ui, /owner\.attachLibraryFile\(file\.handle\)/);
  assert.match(ui, /WebChatLibraryAttachmentReceiptPolicy\.outcome\(/);
  assert.match(ui, /it\.id == result\.requestId/);
  assert.match(ui, /elapsedMs = SystemClock\.elapsedRealtime\(\) - started/);
  assert.match(read('WebChatLibraryAttachmentReceiptPolicy.kt'), /detail == "library_attachment_associated"/);
  assert.match(ui, /host::removeCallbacks/);
  assert.doesNotMatch(ui, /evaluateJavascript|loadUrl|startVoice|setDraft|sendText/);
  assert.match(read('WebChatLibraryBrowser.kt'), /attachments\.start\(port, file\)/);
  assert.match(read('chatgptweb/ChatGptWebMcpActionCatalog.kt'), /"chatgpt_attach_library_file"/);
});

test('production UI routes folders and search to the native owner without dispatching conversation navigation', () => {
  const ui = fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/WebChatLibraryBrowser.kt'), 'utf8');
  const navigation = fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt'), 'utf8');
  assert.match(ui, /item\.kind == "directory"\) navigate\(item\.handle\)/);
  assert.match(ui, /row\.setOnClickListener/);
  assert.match(ui, /page\?\.items\?\.any \{ it\.handle == item\.handle \}/);
  assert.doesNotMatch(ui, /setOnItemClickListener/);
  assert.match(ui, /requestLibraryFiles\(directory, query, operation\)/);
  assert.match(ui, /downloadLibraryFile\(file\.handle, file\.downloadHandle\)/);
  assert.match(ui, /WebChatFileDownloadDialog/);
  assert.match(ui, /setOnDismissListener/);
  assert.doesNotMatch(ui, /openConversation|loadUrl|evaluateJavascript|setDraft|startVoice/);
  assert.match(navigation, /feature\.kind == "library" && library\.show\(\)/);
});

test('production library mutation UI uses confirmed typed commands and keeps a bounded receipt watcher', () => {
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  const ui = read('WebChatLibraryMutationDialog.kt');
  assert.match(ui, /owner\.mutateLibraryFile\(file\.handle, operation, value, true\)/);
  assert.match(ui, /command\.detail == "library_mutation_acknowledged"/);
  assert.match(ui, /started < 30_000/);
  assert.match(ui, /host::removeCallbacks/);
  assert.doesNotMatch(ui, /evaluateJavascript|loadUrl|startVoice|setDraft/);
  assert.match(read('chatgptweb/ChatGptWebMcpActionCatalog.kt'), /"chatgpt_mutate_library_file"/);
  assert.match(read('chatgptweb/ChatGptWebMcpActions.kt'), /in ChatGptWebLibraryCommands\.actions/);
  assert.match(read('chatgptweb/ChatGptWebMcpCommandAdapter.kt'), /pageAdapter\.mutateLibraryFile\(request, requestId\)/);
});
