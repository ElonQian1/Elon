export interface AiSourcePresentationInput {
  title?: string
  url: string
  icon_url?: string
  marker_text?: string
  group_size?: number
  snippet?: string
  thumbnail_url?: string
}

export function safeAiSourceThumbnail(value?: string) {
  return safeIconUrl(value)
}

export interface AiSiteIdentity {
  host: string
  initial: string
  brand: string
  tone: string
}

export function normalizedAiSourceUrl(value: string) {
  const url = publicAiSourceUrl(value)
  if (!url) return value.trim()
  url.search = ''
  url.hash = ''
  url.hostname = url.hostname.toLowerCase().replace(/^www\./, '')
  if (url.pathname.length > 1) url.pathname = url.pathname.replace(/\/+$/, '')
  return url.toString()
}

export function aiSiteIdentity(value: string): AiSiteIdentity {
  try {
    const host = publicAiSourceUrl(value)?.hostname.toLowerCase().replace(/^www\./, '') ?? ''
    if (!host) throw new Error('missing public source host')
    const segments = host.split('.').filter(Boolean)
    const secondLevel = segments[segments.length - 2] || ''
    const suffixIndex = ['co', 'com', 'net', 'org'].includes(secondLevel) ? 3 : 2
    const brand = segments[segments.length - suffixIndex] || segments[0] || 'web'
    return {
      host,
      initial: brand.slice(0, 1).toUpperCase(),
      brand: brandLabel(brand, host),
      tone: String(stableTone(host)),
    }
  } catch {
    return { host: '', initial: 'W', brand: '公开网页', tone: '0' }
  }
}

/**
 * Vendor pages sometimes wrap a public citation in a Google redirect. Resolve only
 * that known public wrapper before dropping tracking data so the native UI can join
 * the visible marker to its structured citation record and publisher favicon.
 */
function publicAiSourceUrl(value: string): URL | undefined {
  try {
    const url = new URL(value)
    if (!isPublicHttpUrl(url)) return undefined
    const host = url.hostname.toLowerCase().replace(/^www\./, '')
    if (host === 'google.com' && url.pathname === '/url') {
      const target = url.searchParams.get('url') || url.searchParams.get('q')
      if (target) {
        const targetUrl = new URL(target)
        if (isPublicHttpUrl(targetUrl)) return targetUrl
      }
    }
    return url
  } catch {
    return undefined
  }
}

function isPublicHttpUrl(url: URL) {
  return ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password
}

export function aiSourceIconCandidates(source: AiSourcePresentationInput, host?: string) {
  const sourceHost = host ?? aiSiteIdentity(source.url).host
  const candidates: string[] = []
  const official = safeIconUrl(source.icon_url)
  if (official && !isIncompleteGoogleFavicon(official)) candidates.push(official)
  const origin = publicAiSourceUrl(source.url)?.origin
  if (origin?.startsWith('https://')) candidates.push(`${origin}/favicon.ico`)
  // The public resolver is intentionally last: it can be unavailable on some
  // Windows networks and must never prevent the publisher's own icon fallback.
  if (sourceHost) candidates.push(googleFaviconUrl(sourceHost))
  return [...new Set(candidates)]
}

export function aiSourceDisplayTitle(
  source: AiSourcePresentationInput,
  identity = aiSiteIdentity(source.url),
) {
  const raw = source.title?.replace(/\s+/g, ' ').trim() ?? ''
  const withoutLeadingUrl = raw.replace(/^https?:\/\/\S+\s*/i, '').trim()
  const readable = withoutLeadingUrl.replace(/\s*\+(\d+)$/, ' +$1')
  return readable && !/^https?:\/\//i.test(readable)
    ? readable
    : identity.brand || identity.host || '公开网页'
}

export function aiInlineCitationLabel(source: AiSourcePresentationInput, visibleText = '') {
  const identity = aiSiteIdentity(source.url)
  const suffix = citationCountSuffix(visibleText)
    || citationCountSuffix(source.marker_text)
    || citationCountSuffix(source.title)
    || (Number.isInteger(source.group_size) && Number(source.group_size) > 1
      ? ` +${Math.min(31, Number(source.group_size) - 1)}`
      : '')
  return `${identity.brand || identity.host || '来源'}${suffix}`
}

function citationCountSuffix(value?: string) {
  const match = value?.trim().match(/\+(\d+)\s*$/)
  return match ? ` +${match[1]}` : ''
}

function safeIconUrl(value?: string) {
  if (!value) return ''
  try {
    const url = new URL(value)
    if (url.protocol !== 'https:' || url.username || url.password) return ''
    return url.toString()
  } catch {
    return ''
  }
}

function isIncompleteGoogleFavicon(value: string) {
  try {
    const url = new URL(value)
    return url.hostname === 'www.google.com'
      && url.pathname === '/s2/favicons'
      && !url.searchParams.has('domain')
      && !url.searchParams.has('domain_url')
  } catch {
    return false
  }
}

function googleFaviconUrl(host: string) {
  const icon = new URL('https://www.google.com/s2/favicons')
  icon.searchParams.set('domain', host)
  icon.searchParams.set('sz', '64')
  return icon.toString()
}

function brandLabel(brand: string, host: string) {
  const knownBrands: Record<string, string> = {
    barrons: "Barron's",
    marketwatch: 'MarketWatch',
    reuters: 'Reuters',
    youtube: 'YouTube',
  }
  if (host.endsWith('sina.com.cn')) return '新浪财经'
  return knownBrands[brand] ?? `${brand.slice(0, 1).toUpperCase()}${brand.slice(1)}`
}

function stableTone(value: string) {
  let total = 0
  for (let index = 0; index < value.length; index += 1) total += value.charCodeAt(index)
  return total % 6
}
