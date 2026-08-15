const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_authentication_policy.js')

assert.equal(policy.isLoginEntry({ label: '登录', href: '' }), true)
assert.equal(policy.isLoginEntry({ label: 'Log in', href: '' }), true)
assert.equal(policy.isLoginEntry({ label: 'Open account settings', href: '/settings' }), false)
assert.equal(policy.isLoginEntry({ label: '', href: '/auth/login?next=%2F' }), true)

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
