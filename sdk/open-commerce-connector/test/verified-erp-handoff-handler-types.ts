import {
  VERIFIED_ERP_HANDOFF_READBACK_SCHEMA,
  createAdapterHandoffWorker,
  createVerifiedErpHandoffHandler,
  type AdapterHandoffClient,
} from '../src/index.js'

interface MerchantOrderResult {
  order: { id: string }
}

const handler = createVerifiedErpHandoffHandler({
  async apply({ source, result }, context) {
    context.signal.throwIfAborted()
    const order = result as MerchantOrderResult
    return {
      targetReference: `${source.merchantId}:${order.order.id}`,
    }
  },
  async readBack({ source, targetReference }) {
    return {
      schema: VERIFIED_ERP_HANDOFF_READBACK_SCHEMA,
      targetReference,
      source,
    }
  },
})

createAdapterHandoffWorker({
  client: {} as AdapterHandoffClient,
  targetDomain: 'erp',
  handler,
})
