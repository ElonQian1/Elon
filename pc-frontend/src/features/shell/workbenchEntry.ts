/** Product navigation is independent of where the workbench assets are hosted. */
export const WORKBENCH_HOME_ROUTE = '/ai'

export function workbenchHomeHref(pathname: string, search = ''): string {
  const base = pathname === '/pc-next' || pathname.startsWith('/pc-next/') ? '/pc-next' : '/pc'
  return `${base}${workbenchHomeRoute(search)}`
}

export function workbenchHomeRoute(search = ''): string {
  const params = new URLSearchParams()
  const nodeAdmin = new URLSearchParams(search).get('node_admin')
  if (nodeAdmin) {
    try {
      const url = new URL(nodeAdmin)
      if (['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)
        && ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password) {
        params.set('node_admin', url.origin + '/')
      }
    } catch { /* Invalid local-node hints are not propagated during recovery. */ }
  }
  const query = params.toString()
  return `${WORKBENCH_HOME_ROUTE}${query ? `?${query}` : ''}`
}
