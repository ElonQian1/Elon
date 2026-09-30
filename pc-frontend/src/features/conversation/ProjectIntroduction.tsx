import { UsersRound, ArrowRight, Sparkles } from 'lucide-react'
import type { Channel, Project, ProjectLanding } from './types'
import styles from './ProjectIntroduction.module.css'

interface Props {
  project: Project
  channels: Channel[]
  landing: ProjectLanding | null
  onSelectChannel: (id: string) => void
  onOpenMembers?: () => void
}

export default function ProjectIntroduction({ project, channels, landing, onSelectChannel, onOpenMembers }: Props) {
  const development = channels.find(channel => channel.kind === 'ai_development')
  const discussion = channels.find(channel => channel.kind === 'discussion')
  const highlights = (landing?.highlights ?? []).filter(Boolean)
  const role = project.role || project.my_role
  const joining = project.join_mode === 'readonly' ? '当前项目提供只读体验，开发操作以授权为准。'
    : project.join_mode === 'open' ? '项目允许开放加入；加入后按角色权限参与协作。'
    : project.join_mode === 'approval' ? '申请通过后加入项目，按角色权限参与协作。'
    : '接受项目邀请后，即可在同一个项目空间按角色权限共同参与。'

  return <section className={styles.introduction} aria-label="项目能力与团队协作">
    <nav className={styles.navigation} aria-label="介绍页导航">
      <a href="#project-capabilities">核心能力</a>
      <a href="#project-collaboration">团队协作</a>
      <a href="#project-details">使用与更新</a>
    </nav>
    <div className={styles.collaboration} id="project-collaboration">
      <div>
        <span className={styles.eyebrow}><UsersRound size={16} aria-hidden="true" />一起参与这个项目</span>
        <h3>{development ? '一起讨论需求，共同开发功能' : '共享项目空间，让协作有据可查'}</h3>
        <p>{joining}</p>
        <p>成员可在授权范围内交流需求、查看项目资料和交付记录{development ? '，并参与 AI 开发' : ''}。管理、发布和节点执行分别受权限与环境约束。</p>
        <div className={styles.actions}>
          {onOpenMembers && <button type="button" onClick={onOpenMembers}>查看团队成员<UsersRound size={16} aria-hidden="true" /></button>}
          {development && <button type="button" onClick={() => onSelectChannel(development.id)}>进入 AI 开发<ArrowRight size={16} aria-hidden="true" /></button>}
          {discussion && <button type="button" onClick={() => onSelectChannel(discussion.id)}>参与项目讨论<ArrowRight size={16} aria-hidden="true" /></button>}
        </div>
        {role && <p className={styles.permission}>你已进入项目空间；可操作范围以当前成员权限为准。</p>}
      </div>
      <ol className={styles.steps}>
        <li><span>01</span><div><strong>加入同一个项目</strong><p>按项目设置接受邀请、申请或加入。</p></div></li>
        <li><span>02</span><div><strong>围绕需求共同参与</strong><p>在项目频道交流，按权限开展工作。</p></div></li>
        <li><span>03</span><div><strong>查看进度与交付</strong><p>通过频道、资料和产物入口追踪结果。</p></div></li>
      </ol>
    </div>
    <section id="project-capabilities" className={styles.capabilities}>
      <span className={styles.eyebrow}><Sparkles size={16} aria-hidden="true" />这个项目能做什么</span>
      <h3>核心能力</h3>
      {highlights.length ? <ul className={styles.grid}>{highlights.map((highlight, index) => <li key={`${index}-${highlight}`}><span>{String(index + 1).padStart(2, '0')}</span><p>{highlight}</p></li>)}</ul>
        : <p>项目尚未提供能力清单，可从项目简介、频道与资料了解当前内容。</p>}
      {!!landing?.target_users?.length && <div className={styles.audience}><strong>适合谁使用</strong><ul>{landing.target_users.map((item, index) => <li key={`${index}-${item}`}>{item}</li>)}</ul></div>}
    </section>
    <section id="project-details" className={styles.details} aria-label="使用与更新详情">
      <Detail title="完整项目介绍" items={landing?.description ? [landing.description] : []} />
      <Detail title="最近更新" items={landing?.recent_updates ?? []} />
      <Detail title="使用条件与环境" items={landing?.system_requirements ?? []} />
      <Detail title="隐私、权限与使用边界" items={landing?.privacy_notes ?? []} />
    </section>
  </section>
}

function Detail({ title, items }: { title: string; items: string[] }) {
  if (!items.length) return null
  return <details><summary>{title}</summary><ul>{items.map((item, index) => <li key={`${index}-${item}`}>{item}</li>)}</ul></details>
}
