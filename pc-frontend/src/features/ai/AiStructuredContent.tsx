import {
  AudioLines,
  Braces,
  FileText,
  Image,
  Map,
  Sigma,
  Table2,
  Video,
  type LucideIcon,
} from 'lucide-react'
import styles from './AiStructuredContent.module.css'
import AiRichContentCard from './AiRichContentCard'
import {
  isYilongRichContent,
  type YilongRichContent,
} from '../user-browser/richContentProtocol'

export interface AiStructuredPart {
  type: 'image' | 'file' | 'code' | 'table' | 'artifact' | 'audio' | 'video' | 'math' | 'chart' | 'map' | 'interactive' | 'rich_card'
  label: string
  kind?: string
  language?: string
  mediaType?: string
  targetHost?: string
  lineCount?: number
  rowCount?: number
  columnCount?: number
  richContent?: YilongRichContent
}

type AiStructuredContentPlacement = 'all' | 'primary' | 'supplementary'

export default function AiStructuredContent({
  parts,
  placement = 'all',
}: {
  parts?: AiStructuredPart[]
  placement?: AiStructuredContentPlacement
}) {
  const allRichParts = parts?.filter((part) => (
    part.type === 'rich_card' && isYilongRichContent(part.richContent)
  )) ?? []
  const richParts = allRichParts.filter((part) => (
    placement === 'all' || (placement === 'primary') === isPrimaryRichCard(part.richContent!)
  ))
  const visibleParts = placement === 'primary' ? [] : parts?.filter((part) => (
    part.type !== 'image' && part.type !== 'rich_card' && Boolean(part.label.trim() || metadataFor(part))
  )) ?? []
  if (!visibleParts.length && !richParts.length) return null
  return (
    <>
      {richParts.map((part, index) => (
        <AiRichContentCard content={part.richContent!} key={`${part.label}:${index}`} />
      ))}
      {visibleParts.length > 0 && (
        <div className={styles.grid} aria-label="官方回复中的结构化内容">
          {visibleParts.map((part, index) => {
            const presentation = presentationFor(part.type)
            const Icon = presentation.icon
            const metadata = metadataFor(part)
            return (
              <article className={styles.card} key={`${part.type}:${part.label}:${index}`}>
                <span className={styles.icon}><Icon size={16} aria-hidden="true" /></span>
                <span className={styles.copy}>
                  <strong>{part.label || presentation.label}</strong>
                  <small>{metadata || presentation.label}</small>
                </span>
              </article>
            )
          })}
        </div>
      )}
    </>
  )
}

function isPrimaryRichCard(content: YilongRichContent) {
  return content.kind === 'finance' || content.kind === 'chart'
    || content.kind === 'weather' || content.kind === 'map'
}

function metadataFor(part: AiStructuredPart) {
  const values = [
    part.language,
    part.mediaType,
    part.targetHost,
    part.lineCount ? `${part.lineCount} 行` : '',
    part.rowCount ? `${part.rowCount} 行 × ${part.columnCount || '?'} 列` : '',
  ].filter(Boolean)
  return values.join(' · ')
}

function presentationFor(type: AiStructuredPart['type']): {
  label: string
  icon: LucideIcon
} {
  switch (type) {
    case 'image': return { label: '图片', icon: Image }
    case 'file': return { label: '文件', icon: FileText }
    case 'code': return { label: '代码', icon: Braces }
    case 'table': return { label: '表格', icon: Table2 }
    case 'audio': return { label: '音频', icon: AudioLines }
    case 'video': return { label: '视频', icon: Video }
    case 'math': return { label: '公式', icon: Sigma }
    case 'map': return { label: '地图', icon: Map }
    default: return { label: '交互内容', icon: Braces }
  }
}
