const test = require('node:test')
const assert = require('node:assert/strict')
const { EventEmitter } = require('node:events')
const { QUERY, readObservation } = require('../graphql')
const { publicLookup } = require('../../esk-sui-publication-observer/transport')

const input = {
  endpoints: ['https://graphql.testnet.sui.io/graphql'],
  participation_package_id: `0x${'11'.repeat(32)}`,
  participation_publication_digest: 'publication-digest',
  allocation_digest: 'allocation-digest',
  allocation_cap_object_id: `0x${'22'.repeat(32)}`,
  allocation_receipt_object_id: `0x${'33'.repeat(32)}`,
  team_vesting_object_id: `0x${'44'.repeat(32)}`,
  allocation_checkpoint_sequence: '42',
  observation_checkpoint_sequence: '51',
}

test('fixed read-only query binds all normalized evidence variables through safe transport', async () => {
  const observation = { chainIdentifier: 'test-chain' }
  let body
  const result = await readObservation(input.endpoints[0], input, {
    request(url, options, callback) {
      assert.equal(url, input.endpoints[0])
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
          res.emit('data', Buffer.from(JSON.stringify({ data: observation })))
          res.emit('end')
        })
      }
      return req
    },
  })

  assert.equal(body.query, QUERY)
  assert.match(QUERY, /^query AllocationObservation/)
  assert.ok(!/\b(mutation|simulateTransaction|executeTransaction)\b/.test(QUERY))
  for (const alias of [
    'participationPublicationTransaction', 'participationPackageObject',
    'allocationTransaction', 'observationCheckpoint', 'receiptAtObservation',
    'vestingAtObservation',
  ]) assert.match(QUERY, new RegExp(`${alias}:`))
  assert.equal(QUERY.match(/objectChanges\(first: 50\)/g).length, 2)
  assert.match(QUERY, /pageInfo \{ hasNextPage hasPreviousPage \}/)
  assert.match(QUERY, /fragment State on Object/)
  assert.match(QUERY, /asMovePackage \{ address version \}/)
  assert.equal(QUERY.match(/lamportVersion/g).length, 2)
  assert.deepEqual(body.variables, {
    participationPackage: input.participation_package_id,
    participationPublication: input.participation_publication_digest,
    allocation: input.allocation_digest,
    receipt: input.allocation_receipt_object_id,
    vesting: input.team_vesting_object_id,
    observationCheckpoint: 51,
  })
  assert.deepEqual(result, observation)
})
