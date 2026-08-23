const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_private_thread_directory.js'),
  'utf8',
)

class FakeXhr {
  constructor() {
    this.listeners = new Map()
    this.status = 0
    this.responseText = ''
  }
  open(method, url) {
    this.method = method
    this.url = url
  }
  send() {}
  addEventListener(name, listener) {
    this.listeners.set(name, listener)
  }
  dispatch(name) {
    const listener = this.listeners.get(name)
    if (listener) listener()
  }
}

const activeId = 'thread_active_123456'
const secondId = 'thread_second_123456'
const location = {
  origin: 'https://www.google.com',
  href: `https://www.google.com/search?q=Current&udm=50&aep=11&csuir=prefix-${activeId}-suffix&mstk=volatile`,
}
const window = { XMLHttpRequest: FakeXhr }
window.window = window
vm.runInNewContext(source, { window, location, URL, JSON, String, Array, Set, WeakMap, Object }, {
  filename: 'google_web_private_thread_directory.js',
})

const directory = window.__elonGoogleWebPrivateThreadDirectory
assert.equal(directory.version, 1)
let changes = 0
directory.setListener(() => { changes += 1 })

const unrelated = new FakeXhr()
unrelated.open('GET', 'https://www.google.com/async/other')
unrelated.status = 200
unrelated.responseText = JSON.stringify([[[activeId, 'Must not import']]])
unrelated.send()
unrelated.dispatch('load')
assert.equal(directory.snapshot().length, 0)

const xhr = new FakeXhr()
xhr.open('GET', 'https://www.google.com/httpservice/web/AimThreadsService/ListThreads')
xhr.status = 200
xhr.responseText = JSON.stringify([[
  [activeId, 'Current title', 'metadata'],
  [secondId, 'Second title', 'metadata'],
]])
xhr.send()
xhr.dispatch('load')

const snapshot = directory.snapshot()
assert.equal(changes, 1)
assert.equal(snapshot.length, 2)
assert.equal(snapshot[0].path, `/c/${activeId}`)
assert.equal(snapshot[1].path, `/c/${secondId}`)
const restored = new URL(snapshot[1].providerUrl)
assert.equal(restored.searchParams.get('q'), 'Second title')
assert.equal(restored.searchParams.get('csuir'), `prefix-${secondId}-suffix`)
assert.equal(restored.searchParams.get('mstk'), 'volatile')
assert.doesNotMatch(source, /document\.cookie|authorization|setRequestHeader|requestBody/)
console.log('GOOGLE_WEB_PRIVATE_THREAD_DIRECTORY_TESTS=passed')
