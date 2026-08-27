import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { CalendarDays, CheckCircle2, ChevronDown, ChevronRight, CircleAlert, Clock3, FolderOpen, Sparkles } from 'lucide-react'
import { v4 as uuidv4 } from 'uuid'
import { useProjectStore } from '../conversation/useProjectStore'
import { saveAiComposerDraft } from '../updates/composerDrafts'
import styles from './AiWorkSummaryPage.module.css'

interface AttentionItem {
  project: string
  title: string
  reason: string
  suggestion: string
  primaryAction: string
  secondaryAction: string
  highPriority?: boolean
  action: 'ai' | 'project' | 'testing'
}

interface ProgressItem {
  project: string
  title: string
  date: string
}

interface ConfirmItem {
  project: string
  title: string
  date: string
}

// APP 当前 AiWorkSummaryActivity 的基线数据。APP 目前也使用本地静态摘要，
// 所以网页先同步 UI/交互，不虚构一个尚未存在的动态接口。
const ATTENTION_ITEMS: AttentionItem[] = [
  {
    project: '一龙网游加速器',
    title: 'Windows 端末检测出新问题',
    reason: '大卫提出了2个兼容性问题\n目前还没有负责人确认',
    suggestion: '建议先确认系统兼容性问题',
    primaryAction: '交给 AI 处理',
    secondaryAction: '进入项目',
    highPriority: true,
    action: 'ai',
  },
  {
    project: '新项目4',
    title: 'APK 构建已完成',
    reason: '等待你是否进入测试阶段。',
    suggestion: '建议先进入测试相关内容',
    primaryAction: '查看项目',
    secondaryAction: '进入测试',
    action: 'testing',
  },
  {
    project: '牛宝',
    title: '主页 UI 修改已完成但未发布',
    reason: '等待你的发布确认',
    suggestion: '建议发布新版本',
    primaryAction: '交给 AI 处理',
    secondaryAction: '查看详情',
    action: 'ai',
  },
]

const PROGRESS_ITEMS: ProgressItem[] = [
  { project: '杀蟑螂', title: '完成了个人页面中心优化', date: '8月17号' },
  { project: '牛宝', title: '修复了交易界面上下自动弹回问题', date: '8月18号' },
]

const CONFIRM_ITEMS: ConfirmItem[] = [
  { project: '大卫', title: '大卫提交发布了新版本。', date: '8月17号' },
]

export default function AiWorkSummaryPage() {
  const navigate = useNavigate()
  const projects = useProjectStore((state) => state.projects)
  const loadProjects = useProjectStore((state) => state.loadProjects)
  const [selectedDate, setSelectedDate] = useState(todayInput())
  const [progressOpen, setProgressOpen] = useState(true)
  const [confirmOpen, setConfirmOpen] = useState(true)
  const [toast, setToast] = useState('')

  useEffect(() => {
    loadProjects().catch(() => {})
  }, [loadProjects])

  useEffect(() => {
    if (!toast) return
    const timer = window.setTimeout(() => setToast(''), 2600)
    return () => window.clearTimeout(timer)
  }, [toast])

  const projectNames = useMemo(() => new Set(projects.map((project) => project.name)), [projects])

  async function openProject(projectName: string) {
    const project = projects.find((candidate) => candidate.name === projectName || candidate.display_name === projectName)
    if (!project) {
      setToast(`暂未找到项目“${projectName}”，已打开项目中心。`)
      navigate('/projects')
      return
    }
    await useProjectStore.getState().selectProject(project.id)
    navigate('/workspace')
  }

  function handoffToAi(item: AttentionItem, testing = false) {
    const prompt = testing
      ? `请为${item.project}进入测试阶段，并先检查 APK 构建结果。`
      : `请处理${item.project}的事项：${item.title}。${item.suggestion}。`
    saveAiComposerDraft({ input: prompt, activeConvId: uuidv4() })
    navigate('/ai')
  }

  function handleAction(item: AttentionItem, primary: boolean) {
    if (!primary && item.action === 'testing') {
      handoffToAi(item, true)
      return
    }
    if (primary && item.action === 'ai') {
      handoffToAi(item)
      return
    }
    void openProject(item.project)
  }

  return (
    <div className={styles.page} data-testid="ai-work-summary-page">
      <header className={styles.header}>
        <div className={styles.titleBlock}>
          <span className={styles.eyebrow}>APP WORK SUMMARY</span>
          <h1>AI 工作摘要</h1>
          <p>按照 APP 当前版本的摘要结构，集中查看项目中的待处理事项。</p>
        </div>
        <label className={styles.datePicker}>
          <span>摘要日期</span>
          <CalendarDays size={16} aria-hidden="true" />
          <input type="date" value={selectedDate} onChange={(event) => setSelectedDate(event.target.value)} aria-label="摘要日期" />
        </label>
      </header>

      <section className={styles.greetingCard}>
        <div>
          <span className={styles.greeting}>早上好</span>
          <h2>今天的工作，从最重要的事情开始。</h2>
          <p>AI 已分析你的 21 个项目</p>
        </div>
        <Sparkles className={styles.greetingIcon} size={34} aria-hidden="true" />
      </section>

      <section className={styles.metrics} aria-label="摘要统计">
        <Metric icon={<CircleAlert size={17} />} label="需要关注" value={ATTENTION_ITEMS.length} tone="warning" />
        <Metric icon={<Clock3 size={17} />} label="有新进展" value={PROGRESS_ITEMS.length} tone="info" />
        <Metric icon={<CheckCircle2 size={17} />} label="待确认" value={CONFIRM_ITEMS.length} tone="success" />
      </section>

      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <div>
            <span className={styles.sectionKicker}>ATTENTION</span>
            <h2>需要关注</h2>
          </div>
          <span className={styles.sectionCount}>{ATTENTION_ITEMS.length} 项</span>
        </div>
        <div className={styles.attentionGrid}>
          {ATTENTION_ITEMS.map((item) => (
            <article className={styles.attentionCard} data-priority={item.highPriority ? 'high' : 'normal'} key={`${item.project}-${item.title}`}>
              <div className={styles.cardTopline}>
                <div className={styles.projectMark}>{item.project.slice(0, 1)}</div>
                <div className={styles.projectCopy}>
                  <strong>{item.project}</strong>
                  <span>{item.title}</span>
                </div>
                {item.highPriority && <span className={styles.priority}>优先处理</span>}
              </div>
              <p className={styles.reason}>{item.reason}</p>
              <div className={styles.suggestion}>
                <Sparkles size={15} aria-hidden="true" />
                <div><span>AI 建议</span><strong>{item.suggestion}</strong></div>
              </div>
              <div className={styles.actions}>
                <button className={item.action === 'ai' ? styles.primaryButton : styles.secondaryButton} type="button" onClick={() => handleAction(item, true)}>
                  {item.action === 'ai' ? <Sparkles size={15} aria-hidden="true" /> : <FolderOpen size={15} aria-hidden="true" />}
                  {item.primaryAction}
                </button>
                <button className={styles.ghostButton} type="button" onClick={() => handleAction(item, false)}>
                  {item.action === 'testing' ? <ChevronRight size={15} aria-hidden="true" /> : <FolderOpen size={15} aria-hidden="true" />}
                  {item.secondaryAction}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>

      <SummarySection title="有新进展" count={PROGRESS_ITEMS.length} open={progressOpen} onToggle={() => setProgressOpen((value) => !value)}>
        {PROGRESS_ITEMS.map((item) => <TimelineRow key={`${item.project}-${item.title}`} project={item.project} title={item.title} date={item.date} projectKnown={projectNames.has(item.project)} />)}
      </SummarySection>

      <SummarySection title="待确认" count={CONFIRM_ITEMS.length} open={confirmOpen} onToggle={() => setConfirmOpen((value) => !value)}>
        {CONFIRM_ITEMS.map((item) => <TimelineRow key={`${item.project}-${item.title}`} project={item.project} title={item.title} date={item.date} projectKnown={projectNames.has(item.project)} />)}
      </SummarySection>

      {toast && <div className={styles.toast} role="status">{toast}</div>}
    </div>
  )
}

function Metric({ icon, label, value, tone }: { icon: React.ReactNode; label: string; value: number; tone: string }) {
  return <div className={styles.metric} data-tone={tone}><span className={styles.metricIcon}>{icon}</span><span>{label}</span><strong>{value}</strong></div>
}

function SummarySection({ title, count, open, onToggle, children }: { title: string; count: number; open: boolean; onToggle: () => void; children: React.ReactNode }) {
  return (
    <section className={styles.summarySection}>
      <button className={styles.summaryHeader} type="button" onClick={onToggle} aria-expanded={open}>
        <span className={styles.summaryChevron}>{open ? <ChevronDown size={17} /> : <ChevronRight size={17} />}</span>
        <strong>{title}</strong>
        <span className={styles.sectionCount}>{count} 项</span>
        <span className={styles.viewAll}>查看全部</span>
      </button>
      {open && <div className={styles.timeline}>{children}</div>}
    </section>
  )
}

function TimelineRow({ project, title, date, projectKnown }: { project: string; title: string; date: string; projectKnown: boolean }) {
  return <div className={styles.timelineRow} data-project-known={projectKnown ? 'true' : 'false'}><div className={styles.timelineDot} /><div className={styles.timelineCopy}><strong>{project}</strong><span>{title}</span></div><time>{date}</time></div>
}

function todayInput() {
  const today = new Date()
  const month = String(today.getMonth() + 1).padStart(2, '0')
  const day = String(today.getDate()).padStart(2, '0')
  return `${today.getFullYear()}-${month}-${day}`
}
