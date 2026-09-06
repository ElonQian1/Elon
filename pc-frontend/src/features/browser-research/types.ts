export const RESULT_SCHEMA = 'yilong.browser-research.result.v1'
export const RESEARCH_KINDS = [
  'sites', 'sessions', 'register_site', 'open', 'status', 'resources', 'search',
  'read_resource', 'requests', 'read_request', 'pause', 'resume',
] as const
export type ResearchKind = typeof RESEARCH_KINDS[number]
export interface SiteManifest {
  schema: 'yilong.browser-research.site.v1'
  id: string
  name: string
  entry_url: string
  navigation_origins: string[]
  resource_origins: string[]
  api_origins: string[]
  identity_origins: string[]
}
export interface ResearchCommand {
  kind: ResearchKind
  site_id?: string
  session_id?: string
  resource_id?: string
  request_id?: string
  query?: string
  offset?: number
  limit?: number
  manifest?: SiteManifest
}
export interface ResearchSession {
  id: string
  site_id: string
  active: boolean
  generation: number
  expires_at_ms: number
  resource_count: number
  request_count: number
  phase: string
  gaps: string[]
  trading_enabled: false
}
export interface ResearchResource {
  id: string
  url: string
  resource_type: string
  mime: string
  size_bytes: number
  sha256: string
  generation: number
  truncated: boolean
  redacted: boolean
}
export interface ResearchRequest {
  id: string
  url: string
  method: string
  status: number | null
  generation: number
  request_resource_id?: string | null
  response_resource_id?: string | null
}
export interface SearchHit {
  resource_id: string
  url: string
  offset: number
  excerpt: string
}
export interface ContentSlice {
  content: string
  offset: number
  next_offset: number | null
  complete: boolean
}
export interface ResultBase { schema: typeof RESULT_SCHEMA }
export interface ResearchList<K extends ResearchKind, T> extends ResultBase {
  kind: K
  items: T[]
  total: number
  offset: number
  next_offset: number | null
  partial?: boolean
}
export type ResearchResult =
  | ResearchList<'sites', SiteManifest>
  | ResearchList<'sessions', ResearchSession>
  | ResearchList<'resources', ResearchResource>
  | ResearchList<'requests', ResearchRequest>
  | ResearchList<'search', SearchHit>
  | ResearchList<'register_site', SiteManifest>
  | (ResultBase & { kind: 'open' | 'status' | 'pause' | 'resume'; session: ResearchSession })
  | (ResultBase & { kind: 'read_resource'; item: ResearchResource } & ContentSlice)
  | (ResultBase & { kind: 'read_request'; request: ResearchRequest; request_body: ContentSlice | null; response_body: ContentSlice | null })
export interface ResearchAction {
  action_id: string
  project_key: string
  command: ResearchCommand
  requested_at_ms: number
  expires_at_ms: number
  status: string
  receipt?: { status: string; result?: unknown; error_code?: string } | null
}
