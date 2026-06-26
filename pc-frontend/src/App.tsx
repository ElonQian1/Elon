import { Routes, Route, Navigate } from 'react-router-dom'
import Shell from './features/shell/Shell'
import LoginPage from './features/auth/LoginPage'
import HomePage from './features/home/HomePage'
import ProjectsPage from './features/projects/ProjectsPage'
import DoctorPage from './features/doctor/DoctorPage'
import VoicePage from './features/voice/VoicePage'
import NodePage from './features/node/NodePage'
import DevTasksPage from './features/dev/DevTasksPage'
import { useAuthStore } from './store/auth'

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const token = useAuthStore((s) => s.token)
  if (!token) return <Navigate to="/login" replace />
  return <>{children}</>
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route
        path="/*"
        element={
          <ProtectedRoute>
            <Shell />
          </ProtectedRoute>
        }
      >
        <Route index element={<HomePage />} />
        <Route path="projects" element={<ProjectsPage />} />
        <Route path="dev-tasks" element={<DevTasksPage />} />
        <Route path="voice" element={<VoicePage />} />
        <Route path="doctor" element={<DoctorPage />} />
        <Route path="node" element={<NodePage />} />
      </Route>
    </Routes>
  )
}
