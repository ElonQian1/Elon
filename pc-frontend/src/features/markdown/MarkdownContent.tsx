import { memo, useCallback, useEffect, useMemo, useState } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type { Components } from 'react-markdown'
import { copyTextToClipboard } from '../../lib/clipboard'
import { MediaViewer, type MediaViewerImage } from './MediaViewer'
import styles from './MarkdownContent.module.css'

interface Props {
  content: string
  /** 是否显示代码块复制按钮（AI 消息默认开启）*/
  copy?: boolean
}

interface MarkdownImage extends MediaViewerImage {
  src: string
  alt: string
  offset?: number
}

type OpenImageViewer = (image: MarkdownImage) => void

function MarkdownContent({ content, copy = true }: Props) {
  const [viewerIndex, setViewerIndex] = useState<number | null>(null)
  const normalizedContent = useMemo(() => normalizeBareImageUrls(content), [content])
  const mediaImages = useMemo(() => extractMarkdownImages(normalizedContent), [normalizedContent])

  useEffect(() => {
    if (viewerIndex !== null && viewerIndex >= mediaImages.length) setViewerIndex(null)
  }, [mediaImages.length, viewerIndex])

  const openImageViewer = useCallback<OpenImageViewer>((image) => {
    setViewerIndex(findImageIndex(mediaImages, image))
  }, [mediaImages])

  const components = useMemo(
    () => buildComponents(copy, openImageViewer),
    [copy, openImageViewer],
  )

  return (
    <div className={styles.root}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={components}
        // 不使用 rehype-sanitize：react-markdown 本身不执行脚本，已足够安全
      >
        {normalizedContent}
      </ReactMarkdown>
      {viewerIndex !== null && (
        <MediaViewer images={mediaImages} index={viewerIndex} onClose={() => setViewerIndex(null)} />
      )}
    </div>
  )
}

export default memo(MarkdownContent)

function buildComponents(
  showCopy: boolean,
  openImageViewer: OpenImageViewer,
): Components {
  return {
    // 代码块（fenced code）
    code({ node: _node, className, children, ...props }) {
      const isBlock = !!className // 有 language-xxx 就是块级
      const codeStr = String(children).replace(/\n$/, '')
      if (!isBlock) {
        return <code className={styles.inlineCode} {...props}>{children}</code>
      }
      return (
        <CodeBlock code={codeStr} className={className} showCopy={showCopy} />
      )
    },
    // 链接：强制新标签，防止 javascript: 等危险协议
    a({ href, children }) {
      const safe = safeMarkdownUrl(href, { image: false })
      return safe
        ? <a href={safe} target="_blank" rel="noopener noreferrer" className={styles.link}>{children}</a>
        : <span className={styles.link}>{children}</span>
    },
    img({ node, src, alt }) {
      const safe = safeMarkdownUrl(src, { image: true })
      if (!safe) return null
      const imageAlt = alt ?? ''
      const image = { src: safe, alt: imageAlt, offset: getNodeOffset(node) }
      const openPreview = () => openImageViewer(image)
      const handleKeyDown = (event: ReactKeyboardEvent<HTMLImageElement>) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          event.stopPropagation()
          openPreview()
        }
      }
      return (
        <img
          src={safe}
          alt={imageAlt}
          className={styles.image}
          loading="lazy"
          role="button"
          tabIndex={0}
          title={imageAlt ? `${imageAlt} - 点击放大` : '点击放大'}
          onClick={(event) => {
            event.preventDefault()
            event.stopPropagation()
            openPreview()
          }}
          onKeyDown={handleKeyDown}
        />
      )
    },
    p({ node, children }) {
      const imageCount = countImageOnlyParagraph(node)
      if (imageCount > 1) {
        return (
          <div className={styles.mediaGrid} data-count={Math.min(imageCount, 4)}>
            {children}
          </div>
        )
      }
      return <p>{children}</p>
    },
    // 表格包裹层（水平滚动）
    table({ children }) {
      return (
        <div className={styles.tableWrap}>
          <table className={styles.table}>{children}</table>
        </div>
      )
    },
    th({ children }) { return <th className={styles.th}>{children}</th> },
    td({ children }) { return <td className={styles.td}>{children}</td> },
    // 段落、标题等保持原有样式，通过 CSS 控制
  }
}

function safeMarkdownUrl(value: string | undefined, options: { image: boolean }): string | undefined {
  if (!value) return undefined
  const url = value.trim()
  if (/^https?:\/\//i.test(url)) return url
  if (!options.image && /^mailto:/i.test(url)) return url
  if (options.image && /^data:image\/(png|jpe?g|gif|webp);base64,/i.test(url)) return url
  if (url.startsWith('/') && !url.startsWith('//')) return url
  return undefined
}

function extractMarkdownImages(value: string): MarkdownImage[] {
  const images: MarkdownImage[] = []
  const imagePattern = /!\[([^\]]*)]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g
  let match: RegExpExecArray | null
  while ((match = imagePattern.exec(value)) !== null) {
    const safe = safeMarkdownUrl(match[2], { image: true })
    if (safe) {
      images.push({
        src: safe,
        alt: decodeMarkdownAlt(match[1]),
        offset: match.index,
      })
    }
  }
  return images
}

function findImageIndex(images: MarkdownImage[], image: MarkdownImage): number {
  if (typeof image.offset === 'number') {
    const byOffset = images.findIndex((item) => item.offset === image.offset)
    if (byOffset >= 0) return byOffset
  }
  const exact = images.findIndex((item) => item.src === image.src && item.alt === image.alt)
  if (exact >= 0) return exact
  const bySrc = images.findIndex((item) => item.src === image.src)
  return bySrc >= 0 ? bySrc : 0
}

function decodeMarkdownAlt(value: string): string {
  return value.replace(/\\([\[\]()])/g, '$1')
}

function getNodeOffset(node: unknown): number | undefined {
  const offset = (node as { position?: { start?: { offset?: unknown } } }).position?.start?.offset
  return typeof offset === 'number' ? offset : undefined
}

function countImageOnlyParagraph(node: unknown): number {
  const children = (node as { children?: Array<{ type?: string; tagName?: string; value?: string }> }).children
  if (!Array.isArray(children)) return 0

  let imageCount = 0
  for (const child of children) {
    if (child.type === 'image' || (child.type === 'element' && child.tagName === 'img')) {
      imageCount += 1
    } else if (child.type === 'text' && (!child.value || child.value.trim() === '')) {
      continue
    } else if (child.type === 'break') {
      continue
    } else {
      return 0
    }
  }
  return imageCount
}

function normalizeBareImageUrls(value: string): string {
  let inFence = false
  return value
    .split(/\r?\n/)
    .map((line) => {
      if (/^\s*```/.test(line)) {
        inFence = !inFence
        return line
      }
      if (inFence) return line
      const match = line.match(/^\s*(https?:\/\/\S+?(?:\.(?:png|jpe?g|gif|webp)|\/(?:chat-)?attachments\/\S+)(?:[?#]\S*)?)\s*$/i)
      return match ? `![](${match[1]})` : line
    })
    .join('\n')
}

/* 独立代码块组件（含复制按钮）*/
function CodeBlock({ code, className, showCopy }: { code: string; className?: string; showCopy: boolean }) {
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle')
  const lang = className?.replace('language-', '') ?? ''

  async function handleCopy() {
    const copied = await copyTextToClipboard(code)
    setCopyState(copied ? 'copied' : 'failed')
    setTimeout(() => setCopyState('idle'), 2000)
  }

  const copyLabel = copyState === 'copied' ? '已复制' : copyState === 'failed' ? '复制失败' : '复制'

  return (
    <div className={styles.codeBlock}>
      {lang && <div className={styles.codeLang}>{lang}</div>}
      {showCopy && (
        <button className={styles.copyBtn} onClick={handleCopy} type="button" data-state={copyState} data-copy-exclude="true">
          {copyLabel}
        </button>
      )}
      <pre className={styles.pre}>
        <code>{code}</code>
      </pre>
    </div>
  )
}
