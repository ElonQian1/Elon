import { Suspense, lazy, type ReactNode } from 'react'
import { Routes, Route } from 'react-router-dom'
import styles from './App.module.css'

const Shell = lazy(() => import('./features/shell/Shell'))
const LoginPage = lazy(() => import('./features/auth/LoginPage'))
const ConversationPage = lazy(() => import('./features/conversation/ConversationPage'))
const ProjectsPage = lazy(() => import('./features/projects/ProjectsPage'))
const ProjectDetailPage = lazy(() => import('./features/projects/ProjectDetailPage'))
const AiChatPage = lazy(() => import('./features/ai/AiChatPage'))
const FriendsPage = lazy(() => import('./features/friends/FriendsPage'))
const PlazaPage = lazy(() => import('./features/plaza/PlazaPage'))
const AccountPage = lazy(() => import('./features/account/AccountPage'))
const DoctorPage = lazy(() => import('./features/doctor/DoctorPage'))
const VoicePage = lazy(() => import('./features/voice/VoicePage'))
const NodePage = lazy(() => import('./features/node/NodePage'))
const DevTasksPage = lazy(() => import('./features/dev/DevTasksPage'))

function RouteFallback() {
  return (
    <div className={styles.routeFallback} role="status" aria-live="polite">
      <span />
      <strong>加载中...</strong>
    </div>
  )
}

function lazyRoute(element: ReactNode) {
  return <Suspense fallback={<RouteFallback />}>{element}</Suspense>
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={lazyRoute(<LoginPage />)} />
      <Route path="/*" element={lazyRoute(<Shell />)}>
        {/* 首页：项目对话主视图 */}
        <Route index element={lazyRoute(<ConversationPage />)} />
        <Route path="ai" element={lazyRoute(<AiChatPage />)} />
        <Route path="friends" element={lazyRoute(<FriendsPage />)} />
        <Route path="plaza" element={lazyRoute(<PlazaPage />)} />
        <Route path="account" element={lazyRoute(<AccountPage />)} />
        <Route path="projects" element={lazyRoute(<ProjectsPage />)} />
        <Route path="projects/:id" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="projects/:id/members" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="dev-tasks" element={lazyRoute(<DevTasksPage />)} />
        <Route path="voice" element={lazyRoute(<VoicePage />)} />
        <Route path="doctor" element={lazyRoute(<DoctorPage />)} />
        <Route path="node" element={lazyRoute(<NodePage />)} />
      </Route>
    </Routes>
  )
}
