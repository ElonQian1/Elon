import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import ProjectLanding from './features/conversation/ProjectLanding'
import type { Channel, ProjectLanding as Landing } from './features/conversation/types'
import manifest from '../../.elon/project-landing.json'
import catalog from '../../server/src/official_project_catalog/catalog.json'
import './styles/globals.css'

/** Development-only fixtures. No API, account, production task or outgoing message. */
function Preview() {
  const [scenario, setScenario] = useState('main')
  const [mode, setMode] = useState('invite')
  const [selection, setSelection] = useState('')
  const source = scenario === 'main' ? manifest
    : scenario === 'child' ? catalog.projects.find(project => project.id === 'yilong-quant')?.landing
    : null
  const landing = source ? {
    ...source,
    ...(scenario === 'main' ? { tagline: '旧节点快照', summary: '旧节点摘要', highlights: ['旧节点能力'] } : {}),
    downloads: Object.entries(source.downloads).map(([platform, value]) => ({ ...value, platform })),
    paper_launch: undefined,
    windows_webview: undefined,
  } as Landing : null
  const channels: Channel[] = scenario === 'empty' ? [] : [
    { id: 'preview-development', name: 'AI 开发', kind: 'ai_development' },
    { id: 'preview-discussion', name: '项目讨论', kind: 'discussion' },
    { id: 'preview-builds', name: '构建与交付', kind: 'builds' },
  ]
  return <>
    <aside style={{ padding: 16, display: 'flex', gap: 12, flexWrap: 'wrap', color: 'var(--text)' }}>
      <strong>离线布局示例，不连接真实项目</strong>
      <label>项目 <select aria-label="预览项目" value={scenario} onChange={event => setScenario(event.target.value)}>
        <option value="main">一龙主项目</option><option value="child">子项目</option><option value="empty">无介绍配置</option>
      </select></label>
      <label>加入方式 <select aria-label="加入方式" value={mode} onChange={event => setMode(event.target.value)}>
        <option value="invite">邀请</option><option value="open">开放加入</option><option value="approval">申请审批</option><option value="readonly">只读体验</option>
      </select></label>
      <output aria-live="polite">{selection && `已选择频道：${selection}`}</output>
    </aside>
    <ProjectLanding project={{ id: scenario === 'main' ? 'elon-self' : 'offline-preview', name: scenario === 'main' ? '一龙 AI' : scenario === 'child' ? '子项目介绍示例' : '尚未配置介绍的项目', role: 'member', join_mode: mode, member_count: 3 }} channels={channels} landing={landing} onSelectChannel={setSelection} onOpenMembers={() => setSelection('members')} />
  </>
}

createRoot(document.getElementById('root')!).render(<Preview />)
