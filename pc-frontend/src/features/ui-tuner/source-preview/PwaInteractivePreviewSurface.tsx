import { MousePointer2, RefreshCw, Smartphone } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { findSourceNode, flattenSourceTree } from './sourcePreviewTree'
import type { SourcePreviewDocument, SourcePreviewNode } from './types'
import styles from './SourcePreview.module.css'

const BRIDGE_SOURCE = 'elon-pwa-design-bridge'
const PARENT_SOURCE = 'elon-pc-ui-tuner'
const PROTOCOL_VERSION = 1

interface PwaIdentity {
  key: string
  uiNode: string
  id: string
  ariaLabel: string
  role: string
  text: string
  tag: string
  classNames: string[]
}

interface PwaSelection {
  identity: PwaIdentity
  rect: { left: number; top: number; width: number; height: number }
}

interface Props {
  url: string
  document: SourcePreviewDocument
  selectedKey: string | null
  zoom: number
  onSelect: (key: string) => void
}

function normalize(value: string | undefined): string {
  return String(value || '')
    .replace(/^@\+?id\//, '')
    .replace(/[^a-z0-9\u4e00-\u9fff]+/gi, '')
    .toLowerCase()
}

function scoreNode(node: SourcePreviewNode, identity: PwaIdentity): number {
  const resourceId = normalize(node.resourceId)
  const name = normalize(node.name)
  const text = normalize(node.style.text)
  const description = normalize(node.style.contentDescription)
  const identities = [identity.uiNode, identity.id, identity.ariaLabel, identity.key].map(normalize).filter(Boolean)
  let score = 0
  if (resourceId && identities.includes(resourceId)) score = Math.max(score, 120)
  if (name && identities.includes(name)) score = Math.max(score, 105)
  if (description && normalize(identity.ariaLabel) === description) score = Math.max(score, 100)
  if (text && normalize(identity.text) === text) score = Math.max(score, 95)
  if (text.length >= 2 && normalize(identity.text).includes(text)) score = Math.max(score, 70)
  if (identity.classNames.some((className) => normalize(className) === resourceId || normalize(className) === name)) score = Math.max(score, 65)
  return score
}

function matchSourceNode(root: SourcePreviewNode, identity: PwaIdentity): SourcePreviewNode | null {
  const ranked = flattenSourceTree(root)
    .map((node) => ({ node, score: scoreNode(node, identity) }))
    .sort((left, right) => right.score - left.score)
  return ranked[0]?.score >= 65 ? ranked[0].node : null
}

function pwaUrl(value: string): string {
  const url = new URL(value, window.location.origin)
  if (url.origin !== window.location.origin) return '/web?ui_tuner_preview=1'
  url.searchParams.set('ui_tuner_preview', '1')
  return `${url.pathname}${url.search}${url.hash}`
}

function previewStyle(node: SourcePreviewNode) {
  return {
    color: node.style.textColor,
    backgroundColor: node.style.background,
    borderRadius: node.style.borderRadius,
    fontSize: node.style.fontSize,
    fontWeight: node.style.fontWeight,
    opacity: node.style.opacity,
    width: node.layout.width,
    height: node.layout.height,
    padding: node.layout.padding,
    margin: node.layout.margin,
  }
}

export function PwaInteractivePreviewSurface({ url, document, selectedKey, zoom, onSelect }: Props) {
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const [ready, setReady] = useState(false)
  const [mode, setMode] = useState<'select' | 'interact'>('select')
  const [selection, setSelection] = useState<PwaSelection | null>(null)
  const [unboundLabel, setUnboundLabel] = useState('')
  const [reloadKey, setReloadKey] = useState(0)
  const selectedNode = useMemo(() => findSourceNode(document.root, selectedKey), [document.root, selectedKey])
  const viewportWidth = Math.max(320, Math.min(430, Math.round(document.canvas.width / 3)))
  const viewportHeight = Math.max(640, Math.min(932, Math.round(document.canvas.height / 3)))
  const scale = Math.max(.55, Math.min(1.5, zoom))

  const post = (type: string, payload: unknown) => {
    iframeRef.current?.contentWindow?.postMessage({ source: PARENT_SOURCE, protocolVersion: PROTOCOL_VERSION, type, payload }, window.location.origin)
  }

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      if (event.origin !== window.location.origin || event.source !== iframeRef.current?.contentWindow) return
      const message = event.data as { source?: string; protocolVersion?: number; type?: string; payload?: { node?: PwaSelection } }
      if (message.source !== BRIDGE_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return
      if (message.type === 'ready') {
        setReady(true)
        post('set-mode', { mode })
      }
      if (message.type === 'selection' && message.payload?.node) {
        const nextSelection = message.payload.node
        setSelection(nextSelection)
        const match = matchSourceNode(document.root, nextSelection.identity)
        if (match) {
          setUnboundLabel('')
          onSelect(match.key)
        } else {
          setUnboundLabel(nextSelection.identity.ariaLabel || nextSelection.identity.text || nextSelection.identity.id || nextSelection.identity.tag)
        }
      }
    }
    window.addEventListener('message', receive)
    return () => window.removeEventListener('message', receive)
  }, [document.root, mode, onSelect])

  useEffect(() => {
    if (ready) post('set-mode', { mode })
  }, [mode, ready])

  useEffect(() => {
    if (!ready || !selection || !selectedNode) return
    post('apply-style', { text: selectedNode.style.text, style: previewStyle(selectedNode) })
  }, [document.root, ready, selectedNode, selection])

  return (
    <div className={styles.pwaPreviewWorkspace} data-testid="pwa-interactive-preview">
      <div className={styles.pwaPreviewToolbar}>
        <span className={ready ? styles.pwaReady : styles.pwaConnecting}><i />{ready ? 'PWA 交互草稿已连接' : '正在连接 PWA 草稿…'}</span>
        <div className={styles.pwaModeSwitch}>
          <button className={mode === 'select' ? styles.activePwaMode : ''} type="button" onClick={() => setMode('select')}><MousePointer2 size={14} />选择组件</button>
          <button className={mode === 'interact' ? styles.activePwaMode : ''} type="button" onClick={() => setMode('interact')}><Smartphone size={14} />操作页面</button>
        </div>
        <button type="button" title="重新载入 PWA 草稿" onClick={() => { setReady(false); setSelection(null); setReloadKey((value) => value + 1) }}><RefreshCw size={14} /></button>
      </div>
      {unboundLabel && <div className={styles.pwaBindingNotice}>已选中“{unboundLabel}”，但它还没有匹配到 Android 节点；可继续操作页面，或让 AI 建立跨端绑定。</div>}
      <div className={styles.pwaDeviceViewport} style={{ width: viewportWidth * scale, height: viewportHeight * scale }}>
        <div className={styles.pwaDraftBadge}>真实 PWA 页面 · Android 最终校准</div>
        <iframe key={reloadKey} ref={iframeRef} className={styles.pwaDeviceFrame} src={pwaUrl(url)} title="移动 PWA 交互草稿" style={{ width: viewportWidth, height: viewportHeight, transform: `scale(${scale})` }} />
      </div>
    </div>
  )
}
