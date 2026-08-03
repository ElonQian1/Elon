import type { SuiAdapterHandoffBundle } from './suiAdapterHandoffTypes'

export function downloadSuiAdapterHandoff(bundle: SuiAdapterHandoffBundle) {
  const blob = new Blob([JSON.stringify(bundle, null, 2)], {
    type: 'application/json;charset=utf-8',
  })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = [
    'sui-handoff',
    bundle.package_kind,
    bundle.target_network,
    bundle.projection_package_id,
  ].join('-') + '.json'
  anchor.click()
  URL.revokeObjectURL(url)
}
