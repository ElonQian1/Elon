import { Routes, Route, Navigate } from 'react-router-dom'
import Shell from './features/shell/Shell'
import LoginPage from './features/auth/LoginPage'
import HomePage from './features/home/HomePage'
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
        {/* 后续各模块路由在此追加 */}
      </Route>
    </Routes>
  )
}
