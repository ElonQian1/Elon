import type { ErpCapability, ErpModule } from './erpBlueprintTypes'

export const starterModules: ErpModule[] = [
  ['catalog', '商品目录'],
  ['order', '订单'],
  ['inventory', '库存'],
  ['customer', '会员客户'],
  ['finance', '经营财务'],
  ['marketing', '营销活动'],
].map(([module_key]) => ({
  module_key,
  version: '1.0.0',
  kind: 'core',
  required: ['catalog', 'order', 'inventory'].includes(module_key),
  dependencies: module_key === 'inventory' ? ['catalog'] : [],
})) as ErpModule[]

export const starterCapabilities: ErpCapability[] = [
  capability('catalog.search', '查询商品', 'catalog', ['搜索商品', '查商品']),
  capability('order.create', '创建订单', 'order', ['下单', '开单']),
  capability('order.status', '查询订单状态', 'order', ['订单进度']),
  capability('inventory.query', '查询库存', 'inventory', ['查库存']),
  capability('inventory.adjust', '调整库存', 'inventory', ['入库', '盘点']),
  capability('customer.member', '管理会员', 'customer', ['会员管理']),
  capability('finance.summary', '经营汇总', 'finance', ['营业汇总', '财务汇总']),
  capability('marketing.campaign', '创建营销活动', 'marketing', ['优惠活动', '营销活动']),
]

export const starterThemes = ['default.clean', 'coffee.warm', 'retail.fresh']
export const starterExtensionPoints = [
  'catalog.enrichment',
  'order.enrichment',
  'marketing.campaign',
  'dashboard.widget',
  'integration.connector',
]

function capability(
  capability_key: string,
  display_name: string,
  module_key: string,
  aliases: string[],
): ErpCapability {
  return {
    capability_key,
    display_name,
    description: display_name,
    category: module_key,
    module_key,
    aliases,
    composable_with: [],
  }
}
