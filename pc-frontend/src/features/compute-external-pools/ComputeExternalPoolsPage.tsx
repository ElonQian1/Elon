import { useState } from 'react'
import { Network, ServerCog, ShieldCheck } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import AdapterReleaseWorkspace from './AdapterReleaseWorkspace'
import OnboardingWorkspace from './OnboardingWorkspace'
import styles from './ComputeExternalPoolsPage.module.css'

type View = 'mine' | 'onboarding-admin' | 'release-admin'

export default function ComputeExternalPoolsPage() {
  const role = useAuthStore((state) => state.user?.role)
  const isAdmin = role === 'admin' || role === 'owner'
  const [view, setView] = useState<View>('mine')
  const effectiveView = isAdmin ? view : 'mine'

  return <main className={styles.page}>
    <header className={styles.pageHeader}>
      <div><span>异构算力来源控制</span><h1>外部算力池</h1><p>登记 Provider 来源与候选 Adapter；激活、路由和任务派发保持关闭。</p></div>
      <div className={styles.boundary}><ShieldCheck size={17} /><span><strong>只登记受控元数据</strong>不读取凭据，不连接外部网络</span></div>
    </header>
    <nav className={styles.viewTabs} aria-label="外部算力池工作区">
      <button type="button" data-active={effectiveView === 'mine'} onClick={() => setView('mine')}><Network size={15} />我的接入申请</button>
      {isAdmin && <button type="button" data-active={effectiveView === 'onboarding-admin'} onClick={() => setView('onboarding-admin')}><ShieldCheck size={15} />Provider 审核</button>}
      {isAdmin && <button type="button" data-active={effectiveView === 'release-admin'} onClick={() => setView('release-admin')}><ServerCog size={15} />Adapter 发布</button>}
    </nav>
    {effectiveView === 'mine' && <OnboardingWorkspace mode="owner" />}
    {effectiveView === 'onboarding-admin' && <OnboardingWorkspace mode="admin" />}
    {effectiveView === 'release-admin' && <AdapterReleaseWorkspace />}
  </main>
}
