import { lazy, Suspense, type ReactNode } from 'react'
import { Navigate, Routes, Route } from 'react-router-dom'
import Shell from './features/shell/Shell'
import { isLocalWorkbench } from './api/runtime'
import styles from './App.module.css'

const LoginPage = lazy(() => import('./features/auth/LoginPage'))
const ConversationPage = lazy(() => import('./features/conversation/ConversationPage'))
const ProjectsPage = lazy(() => import('./features/projects/ProjectsPage'))
const ProjectDetailPage = lazy(() => import('./features/projects/ProjectDetailPage'))
const AiChatPage = lazy(() => import('./features/ai/AiChatPage'))
const FriendsPage = lazy(() => import('./features/friends/FriendsPage'))
const PlazaPage = lazy(() => import('./features/plaza/PlazaPage'))
const AccountPage = lazy(() => import('./features/account/AccountPage'))
const UserProfilePage = lazy(() => import('./features/users/UserProfilePage'))
const DoctorPage = lazy(() => import('./features/doctor/DoctorPage'))
const VoicePage = lazy(() => import('./features/voice/VoicePage'))
const NodePage = lazy(() => import('./features/node/NodePage'))
const PublicDevSmokePage = lazy(() => import('./features/node/PublicDevSmokePage'))
const DevTasksPage = lazy(() => import('./features/dev/DevTasksPage'))
const GitWorktreesPage = lazy(() => import('./features/git-worktrees/GitWorktreesPage'))
const UiTunerPage = lazy(() => import('./features/ui-tuner/UiTunerPage'))
const LocalTasksPage = lazy(() => import('./features/local-tasks/LocalTasksPage'))
const ComputeSettlementPage = lazy(() => import('./features/compute-settlement/ComputeSettlementPage'))
const MyComputeSettlementPage = lazy(() => import('./features/compute-settlement/MyComputeSettlementPage'))
const ComputeSupplyPage = lazy(() => import('./features/compute-supply/ComputeSupplyPage'))
const ComputeActivationAdminPage = lazy(() => import('./features/compute-activation/ComputeActivationAdminPage'))
const ComputeOfferAdminPage = lazy(() => import('./features/compute-offers/ComputeOfferAdminPage'))
const ComputeMarketPage = lazy(() => import('./features/compute-market/ComputeMarketPage'))
const ComputeExecutionPage = lazy(() => import('./features/compute-execution/ComputeExecutionPage'))

function RouteFallback() {
  return <div className={styles.routeFallback} role="status" aria-label="正在加载页面" />
}

function lazyRoute(element: ReactNode) {
  return <Suspense fallback={<RouteFallback />}>{element}</Suspense>
}

export default function App() {
  const defaultPath = isLocalWorkbench() ? '/local-tasks' : '/ai'
  return (
    <Routes>
      <Route path="/login" element={lazyRoute(<LoginPage />)} />
      <Route path="/*" element={<Shell />}>
        {/* 首页：一龙 AI 工作台 */}
        <Route index element={<Navigate to={defaultPath} replace />} />
        <Route path="ai" element={lazyRoute(<AiChatPage />)} />
        <Route path="workspace" element={lazyRoute(<ConversationPage />)} />
        <Route path="friends" element={lazyRoute(<FriendsPage />)} />
        <Route path="plaza" element={lazyRoute(<PlazaPage />)} />
        <Route path="account" element={lazyRoute(<AccountPage />)} />
        <Route path="users/:userId" element={lazyRoute(<UserProfilePage />)} />
        <Route path="projects" element={lazyRoute(<ProjectsPage />)} />
        <Route path="projects/:id" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="projects/:id/members" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="dev-tasks" element={lazyRoute(<DevTasksPage />)} />
        <Route path="git-worktrees" element={lazyRoute(<GitWorktreesPage />)} />
        <Route path="ui-tuner" element={lazyRoute(<UiTunerPage />)} />
        <Route path="local-tasks" element={lazyRoute(<LocalTasksPage />)} />
        <Route path="compute-settlement" element={lazyRoute(<ComputeSettlementPage />)} />
        <Route path="my-compute-settlement" element={lazyRoute(<MyComputeSettlementPage />)} />
        <Route path="compute-supply" element={lazyRoute(<ComputeSupplyPage />)} />
        <Route path="compute-activation" element={lazyRoute(<ComputeActivationAdminPage />)} />
        <Route path="compute-offers" element={lazyRoute(<ComputeOfferAdminPage />)} />
        <Route path="compute-market" element={lazyRoute(<ComputeMarketPage />)} />
        <Route path="compute-execution" element={lazyRoute(<ComputeExecutionPage />)} />
        <Route path="voice" element={lazyRoute(<VoicePage />)} />
        <Route path="doctor" element={lazyRoute(<DoctorPage />)} />
        <Route path="node/public-dev-smoke" element={lazyRoute(<PublicDevSmokePage />)} />
        <Route path="node" element={lazyRoute(<NodePage />)} />
      </Route>
    </Routes>
  )
}
