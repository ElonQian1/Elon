import assert from 'node:assert/strict'
import test from 'node:test'

import {
  CONNECTOR_CONTRACT_VERSION,
  CONNECTOR_SCHEMA,
  ConnectorContractError,
  createSyncReceipt,
  defineConnector,
  runConnectorCompatibility,
  validateManifest,
  validateSyncPage,
  validateSyncRequest,
} from '../src/index.js'

function compliantConnector(overrides = {}) {
  const manifest = {
    schema: CONNECTOR_SCHEMA,
    contractVersion: CONNECTOR_CONTRACT_VERSION,
    connectorKey: 'demo.pos',
    providerKey: 'demo_provider',
    displayName: 'Demo POS',
    connectionMode: 'local_adapter',
    scopes: ['read.orders', 'read.inventory'],
    dataDomains: ['orders', 'inventory'],
    ...overrides.manifest,
  }
  return defineConnector({
    describe: () => manifest,
    health: async () => ({
      status: 'ready',
      observedAt: '2026-07-31T00:00:00Z',
      evidenceCode: 'adapter_authenticated',
    }),
    sync: async (request) => ({
      receiptKey: request.runKey,
      syncKind: request.syncKind,
      status: 'succeeded',
      changes: [
        {
          recordId: 'order-1',
          dataDomain: 'orders',
          operation: 'upsert',
          version: 'v1',
          value: { total_micros: 2_500_000, currency: 'CNY' },
        },
        {
          recordId: 'sku-1',
          dataDomain: 'inventory',
          operation: 'unchanged',
          value: { available: 8 },
        },
      ],
      nextCursor: 'cursor-page-2',
      startedAt: '2026-07-31T00:00:00Z',
      completedAt: '2026-07-31T00:00:01Z',
    }),
  })
}

test('a compliant connector produces a bounded idempotent server receipt', async () => {
  const report = await runConnectorCompatibility(compliantConnector(), {
    request: {
      integrationId: 'integration-demo',
      runKey: 'sync-run-1',
      syncKind: 'incremental',
      dataDomains: ['orders', 'inventory'],
      limit: 100,
    },
  })

  assert.equal(report.compatible, true)
  assert.equal(report.replayVerified, true)
  assert.equal(report.receipt.records_seen, 2)
  assert.equal(report.receipt.records_changed, 1)
  assert.match(report.receipt.cursor_digest, /^sha256:[a-f0-9]{64}$/)
  assert.equal('changes' in report.receipt, false)
})

test('manifest rejects secrets before they enter a development context', () => {
  assert.throws(
    () =>
      validateManifest({
        schema: CONNECTOR_SCHEMA,
        contractVersion: CONNECTOR_CONTRACT_VERSION,
        connectorKey: 'bad.adapter',
        providerKey: 'provider',
        displayName: 'Bad Adapter',
        connectionMode: 'official_api',
        scopes: ['read.orders'],
        dataDomains: ['orders'],
        access_token: 'must-not-leak',
      }),
    (error) =>
      error instanceof ConnectorContractError &&
      error.code === 'sensitive_field' &&
      error.path === 'manifest.access_token',
  )
})

test('sync pages are capped even when an adapter asks for more', () => {
  const request = validateSyncRequest({
    integrationId: 'integration-demo',
    runKey: 'sync-run-2',
    syncKind: 'full',
    dataDomains: ['orders'],
    limit: 1,
  })
  assert.throws(
    () =>
      validateSyncPage(
        {
          receiptKey: 'sync-run-2',
          syncKind: 'full',
          status: 'succeeded',
          changes: [
            {
              recordId: 'order-1',
              dataDomain: 'orders',
              operation: 'upsert',
              value: {},
            },
            {
              recordId: 'order-2',
              dataDomain: 'orders',
              operation: 'upsert',
              value: {},
            },
          ],
          startedAt: '2026-07-31T00:00:00Z',
          completedAt: '2026-07-31T00:00:01Z',
        },
        request,
      ),
    (error) => error instanceof ConnectorContractError && error.code === 'page_too_large',
  )
})

test('receipts expose counts and digests without raw business records', () => {
  const request = validateSyncRequest({
    integrationId: 'integration-demo',
    runKey: 'sync-run-3',
    syncKind: 'incremental',
    dataDomains: ['orders'],
  })
  const receipt = createSyncReceipt(request, {
    receiptKey: 'sync-run-3',
    syncKind: 'incremental',
    status: 'partial',
    changes: [
      {
        recordId: 'order-private',
        dataDomain: 'orders',
        operation: 'upsert',
        value: { customer_name: 'private value' },
      },
    ],
    nextCursor: 'opaque-cursor',
    errorCode: 'source_rate_limited',
    startedAt: '2026-07-31T00:00:00Z',
    completedAt: '2026-07-31T00:00:01Z',
  })

  assert.equal(receipt.records_seen, 1)
  assert.equal(JSON.stringify(receipt).includes('private value'), false)
  assert.equal(receipt.error_code, 'source_rate_limited')
})
