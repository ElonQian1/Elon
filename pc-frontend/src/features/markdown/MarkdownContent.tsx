import { memo, useEffect, useMemo, useState } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import { ExternalLink, X } from 'lucide-react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type { Components } from 'react-markdown'
import styles from './MarkdownContent.module.css'

interface Props {
  content: string
  /** 是否显示代码块复制按钮（AI 消息默认开启）*/
  copy?: boolean
}

interface PreviewImage {
  src: string
  alt: string
}

function MarkdownContent({ content, copy = true }: Props) {
  const [previewImage, setPreviewImage] = useState<PreviewImage | null>(null)
  const normalizedContent = useMemo(() => normalizeBareImageUrls(content), [content])
  useEffect(() => {
    if (!previewImage) return undefined
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setPreviewImage(null)
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      document.body.style.overflow = previousOverflow
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [previewImage])
  const components = useMemo(
    () => buildComponents(copy, setPreviewImage),
    [copy],
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
      {previewImage && (
        <ImagePreviewOverlay image={previewImage} onClose={() => setPreviewImage(null)} />
      )}
    </div>
  )
}

export default memo(MarkdownContent)

function buildComponents(
  showCopy: boolean,
  setPreviewImage: (image: PreviewImage) => void,
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
    img({ src, alt }) {
      const safe = safeMarkdownUrl(src, { image: true })
      if (!safe) return null
      const imageAlt = alt ?? ''
      const openPreview = () => setPreviewImage({ src: safe, alt: imageAlt })
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

function ImagePreviewOverlay({ image, onClose }: { image: PreviewImage; onClose: () => void }) {
  return (
    <div
      className={styles.previewBackdrop}
      role="dialog"
      aria-modal="true"
      aria-label={image.alt || '图片预览'}
      onClick={onClose}
    >
      <div className={styles.previewToolbar} onClick={(event) => event.stopPropagation()}>
        <a
          className={styles.previewIconBtn}
          href={image.src}
          target="_blank"
          rel="noopener noreferrer"
          title="新窗口打开"
        >
          <ExternalLink size={18} aria-hidden="true" />
        </a>
        <button className={styles.previewIconBtn} type="button" onClick={onClose} title="关闭">
          <X size={20} aria-hidden="true" />
        </button>
      </div>
      <img
        src={image.src}
        alt={image.alt}
        className={styles.previewImage}
        onClick={(event) => event.stopPropagation()}
      />
      {image.alt && <div className={styles.previewCaption}>{image.alt}</div>}
    </div>
  )
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
  const [copied, setCopied] = useState(false)
  const lang = className?.replace('language-', '') ?? ''

  async function handleCopy() {
    await navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className={styles.codeBlock}>
      {lang && <div className={styles.codeLang}>{lang}</div>}
      {showCopy && (
        <button className={styles.copyBtn} onClick={handleCopy} type="button">
          {copied ? '已复制' : '复制'}
        </button>
      )}
      <pre className={styles.pre}>
        <code>{code}</code>
      </pre>
    </div>
  )
}
