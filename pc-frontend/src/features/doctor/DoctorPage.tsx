import { useEffect, useRef, useState } from 'react'
import { useDoctorStore } from './useDoctorStore'
import { useAuthStore } from '../../store/auth'
import { formatTime } from '../../lib/utils'
import styles from './DoctorPage.module.css'

const SECTIONS = [
  { id: 'snapshot', icon: '⌁', title: '体检快照' },
  { id: 'router', icon: '⇩', title: '下载加速' },
  { id: 'repair', icon: '✦', title: '修复动作' },
  { id: 'memory', icon: '▱', title: '问题记忆' },
] as const

const QUICK_PROMPTS = [
  '网页打不开，但微信能正常使用',
  '电脑变慢，风扇一直在转',
  '代理关不掉，网络总是异常',
]

export default function DoctorPage() {
  const user = useAuthStore((s) => s.user)
  const {
    sessions, sessionsLoaded, activeSessionId, messages,
    section, result, memories, snapshot, problem, analysis, routerStatus, routerDoctor,
    loadSessions, loadSession, createSession, setSection,
    loadSnapshot, analyze, repair, loadMemory, saveMemory,
    loadRouterStatus, setRouterProfile, runRouterDoctor, aiConfigureRouter,
  } = useDoctorStore()

  const [input, setInput] = useState('')
  const [adapterName, setAdapterName] = useState('')
  const feedRef = useRef<HTMLDivElement>(null)

  useEffect(() => { loadSessions() }, [])
  useEffect(() => { if (section === 'memory' && memories === null) loadMemory() }, [section])
  useEffect(() => { if (section === 'router' && routerStatus === null) loadRouterStatus() }, [section])
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

  async function handleRouterToggle() {
    const enabled = routerStatus?.profile?.enabled ?? true
    await setRouterProfile(!enabled, !enabled ? 'auto' : 'off')
  }

  const routerEnabled = routerStatus?.profile?.enabled ?? true
  const routerMode = routerStatus?.profile?.mode ?? 'auto'

  return (
    <div className={styles.layout}>
      {/* 侧边栏 */}
      <aside className={styles.sidebar}>
        <div className={styles.sidebarBrand}>
          <div>
            <strong>电脑医生</strong>
          </div>
        </div>
        <button className={styles.newSession} onClick={createSession}>
          <span className={styles.newSessionIcon}>+</span>
          <strong>新建诊断</strong>
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
            <span>{sc.title}</span>
          </button>
        ))}
      </aside>

      {/* 主区域 */}
      <div className={styles.main}>
        <header className={styles.header}>
          <div>
            <div className={styles.kicker}>电脑医生 · 本机工作台</div>
            <h2 className={styles.title}>{sessionTitle}</h2>
          </div>
          <div className={styles.headerActions}>
            <button className={styles.actionBtn} onClick={loadSnapshot}>开始体检</button>
            <button className={styles.actionBtn} onClick={() => analyze()} disabled={!problem}>分析当前问题</button>
            <button className={styles.actionBtn} onClick={saveMemory} disabled={!analysis}>保存为记忆</button>
          </div>
        </header>

        <div className={styles.feed} ref={feedRef}>
          {section === 'diagnosis' && (
            <>
              {messages.length === 0 && !result && (
                <div className={styles.emptyState}>
                  <h3>电脑哪里不舒服？</h3>
                  <p>描述你看到的现象，电脑医生会先分析原因，再给出检查或修复建议。</p>
                  <div className={styles.quickPrompts}>
                    {QUICK_PROMPTS.map((prompt) => (
                      <button key={prompt} type="button" onClick={() => setInput(prompt)}>{prompt}</button>
                    ))}
                  </div>
                  <div className={styles.quickPrompts}>
                    <button type="button" onClick={loadSnapshot}>先做一次只读体检</button>
                    <button type="button" onClick={() => setSection('repair')}>查看可用修复动作</button>
                  </div>
                </div>
              )}
              {messages.map((m) => (
                <div key={m.id} className={[styles.msgRow, styles[m.role]].join(' ')}>
                  <div className={styles.avatar}>{m.role === 'user' ? (user?.nickname ?? user?.account)?.[0] ?? '你' : '医'}</div>
                  <div className={styles.msgBody}>
                    <div className={styles.msgMeta}>
                      <strong>{m.role === 'user' ? (user?.nickname ?? user?.account ?? '你') : '电脑医生'}</strong>
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

          {section === 'router' && (
            <div className={styles.routerBox}>
              <div className={styles.routerHeader}>
                <div>
                  <h3>智能下载加速</h3>
                  <span className={routerEnabled ? styles.routerOk : styles.routerOff}>
                    {routerEnabled ? `已启用 · ${routerMode}` : '已关闭'}
                  </span>
                </div>
                <button className={styles.actionBtn} onClick={handleRouterToggle}>
                  {routerEnabled ? '关闭' : '开启'}
                </button>
              </div>

              <div className={styles.modeGrid}>
                {[
                  ['auto', '自动'],
                  ['direct', '直连'],
                  ['system_proxy', '系统代理'],
                  ['off', '关闭'],
                ].map(([mode, label]) => (
                  <button
                    key={mode}
                    className={[styles.modeBtn, routerMode === mode ? styles.modeActive : ''].join(' ')}
                    onClick={() => setRouterProfile(mode !== 'off', mode)}
                  >
                    {label}
                  </button>
                ))}
              </div>

              <div className={styles.routerActions}>
                <button className={styles.repairBtn} onClick={loadRouterStatus}>刷新状态</button>
                <button className={styles.repairBtn} onClick={runRouterDoctor}>运行诊断</button>
                <button className={styles.repairBtn} onClick={aiConfigureRouter}>AI 应用推荐</button>
              </div>

              <div className={styles.routerMeta}>
                <span>Profile</span>
                <code>{routerStatus?.profilePath ?? '未读取'}</code>
              </div>
              <div className={styles.routerMeta}>
                <span>Policy</span>
                <code>{routerStatus?.wrapperPolicy ?? 'PATH wrapper + fail-open'}</code>
              </div>

              {routerDoctor && (
                <div className={styles.snapshotBox}>
                  <pre>{JSON.stringify(routerDoctor, null, 2)}</pre>
                </div>
              )}

              {result && <div className={[styles.resultBox, styles[result.kind]].join(' ')}>{result.text}</div>}
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
