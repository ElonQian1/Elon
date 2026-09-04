const test = require('node:test')
const assert = require('node:assert/strict')
const { EventEmitter } = require('node:events')
const { QUERY, readObservation } = require('../graphql')
const { publicLookup } = require('../../esk-sui-publication-observer/transport')
const { input, observation } = require('./fixtures')

test('fixed read-only query binds every proof variable and safe transport option', async () => {
  const expected = input()
  let body
  const result = await readObservation(expected.endpoints[0], expected, {
    request(url, options, callback) {
      assert.equal(url, expected.endpoints[0])
      assert.equal(options.method, 'POST')
      assert.equal(options.lookup, publicLookup)
      assert.equal(options.rejectUnauthorized, true)
      assert.equal(options.agent, false)
      const req = new EventEmitter()
      req.destroy = () => {}
      req.end = value => {
        body = JSON.parse(value)
        assert.equal(options.headers['content-length'], Buffer.byteLength(value))
        queueMicrotask(() => {
          const res = new EventEmitter()
          res.statusCode = 200
          res.headers = { 'content-type': 'application/json' }
          res.destroy = () => {}
          callback(res)
          res.emit('data', Buffer.from(JSON.stringify({ data: observation() })))
          res.emit('end')
        })
      }
      return req
    },
  })
  assert.equal(body.query, QUERY)
  assert.match(QUERY, /^query CurrencyRegistrationObservation/)
  assert.ok(!/\b(mutation|simulateTransaction|executeTransaction)\b/.test(QUERY))
  assert.deepEqual(body.variables, {
    package: expected.package_id, publication: expected.publication_digest,
    coinType: expected.coin_type, currency: expected.currency_address,
    registration: expected.registration_digest, registrationVersion: Number(expected.registration_version),
  })
  assert.deepEqual(result, observation())
})
