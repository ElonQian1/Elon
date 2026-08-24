const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_authentication_policy.js')

assert.equal(policy.isLoginEntry({ label: '登录', href: '' }), true)
assert.equal(policy.isLoginEntry({ label: 'Log in', href: '' }), true)
assert.equal(policy.isLoginEntry({ label: 'Open account settings', href: '/settings' }), false)
assert.equal(policy.isLoginEntry({ label: '', href: '/auth/login?next=%2F' }), true)

assert.deepEqual(policy.accessDecision({
  pageKind: 'home',
  composerReady: false,
  hasLoginEntry: true,
  visibleText: '登录以继续 Sign in is required to continue. Sign in New chat',
}), {
  blocked: true,
  loginRequired: true,
  reason: 'login_required',
  source: 'visible_page',
})

assert.equal(policy.accessDecision({
  pageKind: 'home',
  composerReady: true,
  hasLoginEntry: true,
  visibleText: '登录 免费注册',
}).blocked, false, 'a normal guest composer with a top-right login entry is not a login wall')

assert.equal(policy.accessDecision({
  pageKind: 'conversation',
  composerReady: false,
  hasLoginEntry: true,
  visibleText: 'Sign in',
}).blocked, false, 'a login button alone is not enough to classify an inline login wall')

assert.equal(policy.accessDecision({ privateStatus: 403 }).reason, 'login_required')
assert.equal(policy.accessDecision({ privateStatus: 429 }).reason, 'rate_limited')
assert.equal(policy.accessDecision({
  composerReady: false,
  hasLoginEntry: true,
  visibleText: '请登录后继续',
  privateStatus: 429,
}).reason, 'login_required', 'visible login requirements outrank passive response hints')

assert.equal(policy.isAuthenticated({
  loginRequired: false,
  hasLoginEntry: true,
  hasProfileEntry: false,
  composerReady: true,
}), false)

assert.equal(policy.isAuthenticated({
  loginRequired: true,
  hasLoginEntry: false,
  hasProfileEntry: true,
  composerReady: false,
}), false)

assert.equal(policy.isAuthenticated({
  loginRequired: false,
  hasLoginEntry: false,
  hasProfileEntry: true,
  composerReady: true,
}), true)

assert.equal(policy.isAuthenticated({
  loginRequired: false,
  hasLoginEntry: false,
  hasProfileEntry: false,
  composerReady: true,
}), true)

console.log('chatgpt web authentication policy passed')
