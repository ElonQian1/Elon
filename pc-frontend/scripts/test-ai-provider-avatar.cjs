const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const avatar = fs.readFileSync(path.join(root, 'src/features/user-browser/AiWebProviderAvatar.tsx'), 'utf8')
const row = fs.readFileSync(path.join(root, 'src/features/ai/AiChatMessageRow.tsx'), 'utf8')
const backend = fs.readFileSync(path.join(root, 'src/features/user-browser/useAiWebChatBackend.ts'), 'utf8')

assert.match(avatar, /providerId === 'chatgpt'/, 'ChatGPT provider avatar mapping is missing')
assert.match(avatar, /providerId === 'google-ai-mode'/, 'Google AI provider avatar mapping is missing')
assert.match(avatar, /return 'ChatGPT'/, 'ChatGPT assistant label is missing')
assert.match(avatar, /return 'Google AI'/, 'Google AI assistant label is missing')
assert.match(row, /<AiWebProviderAvatar providerId=\{message\.assistant_provider_id\}/, 'message row does not render the provider avatar')
assert.match(row, /<UserAvatar user=\{user\}/, 'user avatar must remain unchanged')
assert.match(backend, /assistant_provider_id: provider\?\.id/, 'web messages do not retain their provider identity')

console.log('PASS: production AI messages use provider-specific avatars while user and work-mode avatars remain unchanged')
