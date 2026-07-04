import { memo, useMemo, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type { Components } from 'react-markdown'
import styles from './MarkdownContent.module.css'

interface Props {
  content: string
  /** 是否显示代码块复制按钮（AI 消息默认开启）*/
  copy?: boolean
}

function MarkdownContent({ content, copy = true }: Props) {
  const components = useMemo(() => buildComponents(copy), [copy])
  return (
    <div className={styles.root}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={components}
        // 不使用 rehype-sanitize：react-markdown 本身不执行脚本，已足够安全
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}

export default memo(MarkdownContent)

function buildComponents(showCopy: boolean): Components {
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
      return (
        <a href={safe} target="_blank" rel="noopener noreferrer" className={styles.imageLink}>
          <img src={safe} alt={alt ?? ''} className={styles.image} loading="lazy" />
        </a>
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

function safeMarkdownUrl(value: string | undefined, options: { image: boolean }): string | undefined {
  if (!value) return undefined
  const url = value.trim()
  if (/^https?:\/\//i.test(url)) return url
  if (!options.image && /^mailto:/i.test(url)) return url
  if (options.image && /^data:image\/(png|jpe?g|gif|webp);base64,/i.test(url)) return url
  if (url.startsWith('/') && !url.startsWith('//')) return url
  return undefined
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
