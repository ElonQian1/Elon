import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { registerPcServiceWorker } from './registerPcServiceWorker'
import WorkbenchErrorBoundary from './WorkbenchErrorBoundary'
import './styles/globals.css'

const App = React.lazy(() => import('./App'))

registerPcServiceWorker()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <WorkbenchErrorBoundary>
      <BrowserRouter basename="/pc">
        <React.Suspense fallback={<div role="status" aria-label="正在加载工作台" />}>
          <App />
        </React.Suspense>
      </BrowserRouter>
    </WorkbenchErrorBoundary>
  </React.StrictMode>,
)
