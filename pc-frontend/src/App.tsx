import { Routes, Route } from 'react-router-dom'
import Shell from './features/shell/Shell'
import LoginPage from './features/auth/LoginPage'
import ConversationPage from './features/conversation/ConversationPage'
import ProjectsPage from './features/projects/ProjectsPage'
import ProjectDetailPage from './features/projects/ProjectDetailPage'
import AiChatPage from './features/ai/AiChatPage'
import FriendsPage from './features/friends/FriendsPage'
import PlazaPage from './features/plaza/PlazaPage'
import AccountPage from './features/account/AccountPage'
import DoctorPage from './features/doctor/DoctorPage'
import VoicePage from './features/voice/VoicePage'
import NodePage from './features/node/NodePage'
import DevTasksPage from './features/dev/DevTasksPage'

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/*" element={<Shell />}>
        {/* 首页：项目对话主视图 */}
        <Route index element={<ConversationPage />} />
        <Route path="ai" element={<AiChatPage />} />
        <Route path="friends" element={<FriendsPage />} />
        <Route path="plaza" element={<PlazaPage />} />
        <Route path="account" element={<AccountPage />} />
        <Route path="projects" element={<ProjectsPage />} />
        <Route path="projects/:id" element={<ProjectDetailPage />} />
        <Route path="projects/:id/members" element={<ProjectDetailPage />} />
        <Route path="dev-tasks" element={<DevTasksPage />} />
        <Route path="voice" element={<VoicePage />} />
        <Route path="doctor" element={<DoctorPage />} />
        <Route path="node" element={<NodePage />} />
      </Route>
    </Routes>
  )
}
