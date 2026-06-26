import { useEffect, useRef, useState } from 'react'
import { useDoctorStore } from './useDoctorStore'
import { useAuthStore } from '../../store/auth'
import { formatTime } from '../../lib/utils'
import styles from './DoctorPage.module.css'

const SECTIONS = [
  { id: 'snapshot', icon: '查', title: '体检快照' },
  { id: 'repair', icon: '修', title: '修复动作' },
  { id: 'memory', icon: '记', title: '问题记忆' },
] as const

export default function DoctorPage() {
  const user = useAuthStore((s) => s.user)
  const {
    sessions, sessionsLoaded, activeSessionId, messages,
    section, result, memories, snapshot, problem, analysis,
    loadSessions, loadSession, createSession, setSection,
    loadSnapshot, analyze, repair, loadMemory, saveMemory,
  } = useDoctorStore()

  const [input, setInput] = useState('')
  const [adapterName, setAdapterName] = useState('')
  const feedRef = useRef<HTMLDivElement>(null)

  useEffect(() => { loadSessions() }, []) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (section === 'memory' && memories === null) loadMemory() }, [section]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (feedRef.current) feedRef.current.scrollTop = feedRef.current.scrollHeight
  }, [messages, result])

  const sessionTitle = (sessions.find((s) => s.id === activeSessionId)?.title
    ?? problem) || '新的电脑诊断'

  async function handleSend(e: React.FormEvent) {
    e.preventDefault()
    const text = input.trim()
    if (!text) return
    setInput('')
    await analyze(text)
  }

  async function handleRepair(action: string) {
    if (action === 'restart_adapter' && !adapterName.trim()) {
      setSection('repair')
      useDoctorStore.setState({ result: { kind: 'err', text: '请先填写网卡名称。' } })
      return
    }
    const labels: Record<string, string> = {
      flush_dns: '清 DNS 缓存',
      reset_winhttp_proxy: '重置 WinHTTP 代理',
      clear_user_proxy: '关闭当前用户代理',
      restart_adapter: `重启网卡：${adapterName}`,
    }
    if (!window.confirm(`确认执行「${labels[action] ?? action}」？该动作会修改本机网络状态。`)) return
    await repair(action, adapterName.trim() || undefined)
  }

  return (
    <div className={styles.layout}>
      {/* 侧边栏 */}
      <aside className={styles.sidebar}>
        <div className={styles.sideSection}>电脑医生项目</div>
        <button className={styles.newSession} onClick={createSession}>
          <span>+</span> 新诊断
        </button>

        <div className={styles.sideSection}>诊断会话</div>
        {!sessionsLoaded && <p className={styles.sideEmpty}>正在读取会话…</p>}
        {sessionsLoaded && sessions.length === 0 && <p className={styles.sideEmpty}>暂无诊断会话</p>}
        {sessions.map((s) => (
          <button
            key={s.id}
            className={[styles.sessionBtn, s.id === activeSessionId ? styles.sessionActive : ''].join(' ')}
            onClick={() => loadSession(s.id)}
          >
            <span className={styles.sessionIcon}>#</span>
            <span className={styles.sessionMeta}>
              <strong>{s.title || '未命名诊断'}</strong>
              <small>
                {s.messageCount ? `${s.messageCount} 条消息 · ` : ''}
                {s.updatedAtMs ? formatTime(s.updatedAtMs) : ''}
              </small>
            </span>
          </button>
        ))}

        <div className={styles.sideSection}>工具</div>
        {SECTIONS.map((sc) => (
          <button
            key={sc.id}
            className={[styles.toolBtn, section === sc.id ? styles.toolActive : ''].join(' ')}
            onClick={() => setSection(sc.id)}
          >
            <span className={styles.toolIcon}>{sc.icon}</span>
            {sc.title}
          </button>
        ))}
      </aside>

      {/* 主区域 */}
      <div className={styles.main}>
        <header className={styles.header}>
          <div>
            <div className={styles.kicker}>Windows PC Doctor Session</div>
            <h2 className={styles.title}>{sessionTitle}</h2>
          </div>
          <div className={styles.headerActions}>
            <button className={styles.actionBtn} onClick={loadSnapshot}>只读体检</button>
            <button className={styles.actionBtn} onClick={() => analyze()} disabled={!problem}>分析最近问题</button>
            <button className={styles.actionBtn} onClick={saveMemory} disabled={!analysis}>保存记忆</button>
          </div>
        </header>

        <div className={styles.feed} ref={feedRef}>
          {section === 'diagnosis' && (
            <>
              {messages.length === 0 && !result && (
                <div className={styles.emptyHint}>
                  用底部的消息发送框描述电脑问题，例如"网页打不开但微信能用"或"代理关不掉"。
                </div>
              )}
              {messages.map((m) => (
                <div key={m.id} className={[styles.msgRow, styles[m.role]].join(' ')}>
                  <div className={styles.avatar}>{m.role === 'user' ? (user?.display_name?.[0] ?? '你') : '医'}</div>
                  <div className={styles.msgBody}>
                    <div className={styles.msgMeta}>
                      <strong>{m.role === 'user' ? (user?.display_name ?? '你') : '电脑医生'}</strong>
                      <span>{m.time}</span>
                    </div>
                    <pre className={[styles.msgContent, styles[m.kind] ?? ''].join(' ')}>{m.content}</pre>
                  </div>
                </div>
              ))}
              {result && (
                <div className={[styles.resultBox, styles[result.kind]].join(' ')}>{result.text}</div>
              )}
            </>
          )}

          {section === 'snapshot' && (
            <div className={styles.snapshotBox}>
              {snapshot
                ? <pre>{JSON.stringify(snapshot, null, 2)}</pre>
                : <p>尚未采集体检快照，点击「只读体检」开始。</p>}
            </div>
          )}

          {section === 'repair' && (
            <div className={styles.repairBox}>
              <h3>白名单修复动作</h3>
              <label className={styles.repairField}>
                <span>网卡名称（重启网卡时使用）</span>
                <input value={adapterName} onChange={(e) => setAdapterName(e.target.value)} placeholder="如 Wi-Fi" />
              </label>
              {(['flush_dns', 'reset_winhttp_proxy', 'clear_user_proxy', 'restart_adapter'] as const).map((a) => (
                <button key={a} className={styles.repairBtn} onClick={() => handleRepair(a)}>
                  {{ flush_dns: '清 DNS 缓存', reset_winhttp_proxy: '重置 WinHTTP 代理', clear_user_proxy: '关闭当前用户代理', restart_adapter: '重启指定网卡' }[a]}
                </button>
              ))}
              {result && <div className={[styles.resultBox, styles[result.kind]].join(' ')}>{result.text}</div>}
            </div>
          )}

          {section === 'memory' && (
            <div className={styles.memoryBox}>
              {!memories && <p>正在读取…</p>}
              {memories?.length === 0 && <p>暂无问题记忆。</p>}
              {memories?.map((item, i) => (
                <div key={i} className={styles.memoryItem}>
                  <strong>{item.problem}</strong>
                  <span>{item.summary}</span>
                  {item.createdAtMs && <time>{formatTime(item.createdAtMs)}</time>}
                </div>
              ))}
            </div>
          )}
        </div>

        <form className={styles.composer} onSubmit={handleSend}>
          <input
            className={styles.composerInput}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="描述电脑问题，按 Enter 发送给电脑医生"
          />
          <button className={styles.sendBtn} type="submit">发送</button>
        </form>
      </div>
    </div>
  )
}
