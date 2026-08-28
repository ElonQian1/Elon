import type { YilongRichContent } from './richContentProtocol'

export type LocalAiStructuredPartType =
  | 'image'
  | 'file'
  | 'citation'
  | 'code'
  | 'table'
  | 'artifact'
  | 'audio'
  | 'video'
  | 'math'
  | 'chart'
  | 'map'
  | 'interactive'
  | 'rich_card'

interface LocalAiStructuredContentMetadata {
  text: string
  url?: string
  iconUrl?: string
  markerText?: string
  citationId?: string
  groupSize?: number
  snippet?: string
  thumbnailUrl?: string
  kind?: string
  language?: string
  mediaType?: string
  targetKind?: 'same_origin' | 'external'
  targetHost?: string
  lineCount?: number
  rowCount?: number
  columnCount?: number
  richContent?: YilongRichContent
}

export type LocalAiStructuredContentPart = {
  [PartType in LocalAiStructuredPartType]: LocalAiStructuredContentMetadata & { type: PartType }
}[LocalAiStructuredPartType]

export interface LocalAiAttachment {
  id: string
  name: string
  state: 'uploading' | 'ready' | 'error'
  removable: boolean
}

export interface LocalAiComposerOption {
  id: string
  label: string
  selected: boolean
  kind: string
  semantic: string
  opensSubmenu: boolean
}

export interface LocalAiComposerControlsSnapshot {
  type: 'composer_controls_snapshot'
  section: 'model' | 'tools'
  currentModel: string
  options: LocalAiComposerOption[]
}

export interface LocalAiFeatureNavigationItem {
  id: string
  label: string
  kind: string
  selected: boolean
}

export interface LocalAiFeatureNavigationSnapshot {
  type: 'navigation_snapshot'
  features: LocalAiFeatureNavigationItem[]
}

export interface LocalAiUiControl {
  id: string
  semantic: string
  label: string
  region: 'header' | 'suggestions' | 'composer' | 'overlay' | 'message' | 'content'
  role: string
  enabled: boolean
  selected: boolean
}

export interface LocalAiUiManifestSnapshot {
  type: 'ui_manifest_snapshot'
  version: number
  pageKind: string
  title: string
  compatibility: 'healthy' | 'partial' | 'fallback_required'
  controls: LocalAiUiControl[]
  discoveredControlCount: number
  controlsTruncated: boolean
}

export interface LocalAiSessionDiagnostics {
  lastEventKind: string
  lastCommandAction: string
  lastCommandRequestId?: string | null
  lastCommandOk?: boolean | null
  messageCount: number
  assistantMessageCount: number
  contentPartCounts?: Record<string, number>
  richCardKindCounts?: Record<string, number>
  citationCount?: number
  linkedCitationCount?: number
  citationLogoCount?: number
  streaming: boolean
  privateStreamObserved?: boolean
  privateStreamRevision?: number
  privateStreamState?: 'idle' | 'streaming' | 'completed'
  privateRichRecovery?: LocalAiPrivateRichRecovery
  updatedAtMs: number
}

export interface LocalAiPrivateRichRecovery {
  version: number
  generation: number
  active: boolean
  detached: boolean
  conversationBound: boolean
  turnBound: boolean
  messageBound: boolean
  richKinds: string[]
  acceptedCount: number
  rejectedCount: number
  lastOutcome: 'none' | 'reset' | 'accepted' | 'accepted_detached' | 'invalid' | 'empty' | 'stale_generation' | 'route_mismatch' | 'detached_incomplete' | 'identity_mismatch' | 'expired'
  placeholderReconciled: boolean
  sampledAtMs: number
}

export function isLocalAiComposerControlsSnapshot(
  value: unknown,
): value is LocalAiComposerControlsSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const snapshot = value as Partial<LocalAiComposerControlsSnapshot>
  return snapshot.type === 'composer_controls_snapshot'
    && (snapshot.section === 'model' || snapshot.section === 'tools')
    && Array.isArray(snapshot.options)
}

export function isLocalAiFeatureNavigationSnapshot(
  value: unknown,
): value is LocalAiFeatureNavigationSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const snapshot = value as Partial<LocalAiFeatureNavigationSnapshot>
  return snapshot.type === 'navigation_snapshot' && Array.isArray(snapshot.features)
}

export function isLocalAiUiManifestSnapshot(value: unknown): value is LocalAiUiManifestSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const snapshot = value as Partial<LocalAiUiManifestSnapshot>
  return snapshot.type === 'ui_manifest_snapshot' && Array.isArray(snapshot.controls)
}
