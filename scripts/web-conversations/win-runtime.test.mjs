import { test } from 'node:test'
import assert from 'node:assert/strict'
import { connectWin } from './win-runtime.mjs'

function fixture({ node = true, hosts = [], reader = true } = {}) {
  let time = 0, launches = 0, polls = 0
  const deps = {
    now: () => time,
    sleep: async ms => { time += ms },
    discover: async () => node ? 'http://127.0.0.1:7799' : null,
    launch: async () => { launches++; node = true; hosts = [{ instance_id: 'started-host' }] },
    bootstrap: async () => async action => {
      if (action === 'describe') return { commands: reader ? { read_conversation: {} } : {} }
      if (action === 'hosts') { polls++; return { hosts } }
      throw Error('unexpected_action')
    },
  }
  return { deps, launches: () => launches, polls: () => polls }
}
test('cold Win starts installed launcher once then waits for both node and desktop', async () => {
  const f = fixture({ node: false })
  const result = await connectWin({}, '.', f.deps)
  assert.equal(result.instance, 'started-host'); assert.equal(f.launches(), 1)
})
test('running node without desktop starts desktop; online desktop is reused', async () => {
  const f = fixture(), g = fixture({ hosts: [{ instance_id: 'existing' }] })
  assert.equal((await connectWin({}, '.', f.deps)).instance, 'started-host')
  assert.equal(f.launches(), 1)
  assert.equal((await connectWin({}, '.', g.deps)).instance, 'existing')
  assert.equal(g.launches(), 0)
})
test('old runtime, ambiguous hosts, disabled start and missing pinned host fail explicitly', async () => {
  const cases = [
    [fixture({ reader: false }), {}, /win_reader_update_required/],
    [fixture({ hosts: [{ instance_id: 'a' }, { instance_id: 'b' }] }), {}, /win_host_selection_required/],
    [fixture(), { ELON_WEB_CONVERSATION_AUTOSTART: '0' }, /win_host_offline/],
    [fixture(), { ELON_WEB_CONVERSATION_WIN_INSTANCE: 'previous' }, /win_host_offline/],
  ]
  for (const [f, env, error] of cases) {
    await assert.rejects(connectWin(env, '.', f.deps), error); assert.equal(f.launches(), 0)
  }
})
test('startup wait is bounded and never relaunches repeatedly', async () => {
  for (const layer of ['node', 'host']) {
    const f = fixture({ node: layer !== 'node' })
    let launches = 0
    f.deps.launch = async () => { launches++ }
    await assert.rejects(connectWin({}, '.', f.deps), new RegExp(`win_${layer}_start_timeout`))
    assert.equal(launches, 1)
  }
})
