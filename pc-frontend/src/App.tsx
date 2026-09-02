import { lazy, Suspense, type ReactNode } from 'react'
import { Navigate, Routes, Route } from 'react-router-dom'
import Shell from './features/shell/Shell'
import ComputeWorkspaceLayout from './features/compute/ComputeWorkspaceLayout'
import { isLocalWorkbench } from './api/runtime'
import styles from './App.module.css'

const LoginPage = lazy(() => import('./features/auth/LoginPage'))
const ConversationPage = lazy(() => import('./features/conversation/ConversationPage'))
const ProjectsPage = lazy(() => import('./features/projects/ProjectsPage'))
const ProjectDetailPage = lazy(() => import('./features/projects/ProjectDetailPage'))
const AiHomePage = lazy(() => import('./features/ai/AiHomePage'))
const AiWorkSummaryPage = lazy(() => import('./features/ai/AiWorkSummaryPage'))
const FriendsPage = lazy(() => import('./features/friends/FriendsPage'))
const PlazaPage = lazy(() => import('./features/plaza/PlazaPage'))
const AccountPage = lazy(() => import('./features/account/AccountPage'))
const EskAssetCard = lazy(() => import('./features/assets/EskAssetCard'))
const EskAssetPreview = lazy(async () => {
  const data = await import('./features/assets/eskAssetApi')
  return {
    default: () => <EskAssetCard
      previewMode
      initialSnapshot={data.ESK_PREVIEW_SNAPSHOT}
      initialRequests={data.ESK_PREVIEW_REQUESTS}
      initialQuantRequests={data.ESK_PREVIEW_QUANT_REQUESTS}
    />,
  }
})
const UserBrowserLauncherPage = lazy(() => import('./features/user-browser/UserBrowserLauncherPage'))
const OpenAiChatKitPage = lazy(() => import('./features/chatkit/OpenAiChatKitPage'))
const UserProfilePage = lazy(() => import('./features/users/UserProfilePage'))
const DoctorPage = lazy(() => import('./features/doctor/DoctorPage'))
const VoicePage = lazy(() => import('./features/voice/VoicePage'))
const QuantPaperLaunch = lazy(() => import('./features/conversation/QuantPaperLaunch'))
const NodePage = lazy(() => import('./features/node/NodePage'))
const PublicDevSmokePage = lazy(() => import('./features/node/PublicDevSmokePage'))
const DevTasksPage = lazy(() => import('./features/dev/DevTasksPage'))
const GitWorktreesPage = lazy(() => import('./features/git-worktrees/GitWorktreesPage'))
const UiTunerPage = lazy(() => import('./features/ui-tuner/UiTunerPage'))
const LocalTasksPage = lazy(() => import('./features/local-tasks/LocalTasksPage'))
const CodexControlPage = lazy(() => import('./features/codex-control/CodexControlPage'))
const ComputeSettlementPage = lazy(() => import('./features/compute-settlement/ComputeSettlementPage'))
const MyComputeSettlementPage = lazy(() => import('./features/compute-settlement/MyComputeSettlementPage'))
const ComputeSupplyPage = lazy(() => import('./features/compute-supply/ComputeSupplyPage'))
const ComputeActivationAdminPage = lazy(() => import('./features/compute-activation/ComputeActivationAdminPage'))
const ComputeOfferAdminPage = lazy(() => import('./features/compute-offers/ComputeOfferAdminPage'))
const ComputeReferenceCurvePage = lazy(() => import('./features/compute-reference-curves/ComputeReferenceCurvePage'))
const ComputeExternalPoolsPage = lazy(() => import('./features/compute-external-pools/ComputeExternalPoolsPage'))
const ComputeMarketPage = lazy(() => import('./features/compute-market/ComputeMarketPage'))
const ComputeConsumerReviewPage = lazy(() => import('./features/compute-market/ComputeConsumerReviewPage'))
const ComputeExecutionPage = lazy(() => import('./features/compute-execution/ComputeExecutionPage'))
const ComputePlatformObservationPage = lazy(() => import('./features/compute-observations/ComputePlatformObservationPage'))
const ComputeVerificationPage = lazy(() => import('./features/compute-verification/ComputeVerificationPage'))
const ComputeExecutionReceiptPage = lazy(() => import('./features/compute-receipts/ComputeExecutionReceiptPage'))
const ComputeAttemptFinalizationPage = lazy(() => import('./features/compute-finalization/ComputeAttemptFinalizationPage'))
const ComputeSettlementIssuancePage = lazy(() => import('./features/compute-settlement/ComputeSettlementIssuancePage'))
const ComputeSettlementChallengePage = lazy(() => import('./features/compute-settlement/ComputeSettlementChallengePage'))
const ComputeSettlementChallengeResolutionPage = lazy(() => import('./features/compute-settlement/ComputeSettlementChallengeResolutionPage'))
const ComputeSettlementCorrectionPage = lazy(() => import('./features/compute-settlement/ComputeSettlementCorrectionPage'))

function RouteFallback() {
  return <div className={styles.routeFallback} role="status" aria-label="正在加载页面" />
}

function lazyRoute(element: ReactNode) {
  return <Suspense fallback={<RouteFallback />}>{element}</Suspense>
}

export default function App() {
  if (import.meta.env.DEV && new URLSearchParams(window.location.search).get('ui_preview') === 'esk-asset') {
    return <main className={styles.uiPreviewSurface}>{lazyRoute(<EskAssetPreview />)}</main>
  }
  if (import.meta.env.DEV && new URLSearchParams(window.location.search).get('ui_preview') === 'quant-paper-launch') {
    return (
      <main className={styles.uiPreviewSurface}>
        <Suspense fallback={<RouteFallback />}>
          <QuantPaperLaunch
            previewMode="ready"
            integration={{
              schema: 'yilong.quant.paper_launch.v1',
              mode: 'paper',
              label: '进入 Paper 模拟持仓',
              description: '由一龙账号签发五分钟短期授权，在内存通道中打开本人模拟仓位。',
              simulated: true,
              funds_moved: false,
              target_is_guaranteed: false,
            }}
          />
        </Suspense>
      </main>
    )
  }
  const defaultPath = isLocalWorkbench() ? '/local-tasks' : '/ai'
  return (
    <Routes>
      <Route path="/login" element={lazyRoute(<LoginPage />)} />
      <Route path="/*" element={<Shell />}>
        {/* 首页：一龙 AI 工作台 */}
        <Route index element={<Navigate to={defaultPath} replace />} />
        <Route path="ai" element={lazyRoute(<AiHomePage />)} />
        <Route path="ai-work-summary" element={lazyRoute(<AiWorkSummaryPage />)} />
        <Route path="workspace" element={lazyRoute(<ConversationPage />)} />
        <Route path="friends" element={lazyRoute(<FriendsPage />)} />
        <Route path="plaza" element={lazyRoute(<PlazaPage />)} />
        <Route path="account" element={lazyRoute(<AccountPage />)} />
        <Route path="user-browser" element={lazyRoute(<UserBrowserLauncherPage />)} />
        <Route path="chatkit" element={lazyRoute(<OpenAiChatKitPage />)} />
        <Route path="users/:userId" element={lazyRoute(<UserProfilePage />)} />
        <Route path="projects" element={lazyRoute(<ProjectsPage />)} />
        <Route path="projects/:id" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="projects/:id/members" element={lazyRoute(<ProjectDetailPage />)} />
        <Route path="dev-tasks" element={lazyRoute(<DevTasksPage />)} />
        <Route path="git-worktrees" element={lazyRoute(<GitWorktreesPage />)} />
        <Route path="ui-tuner" element={lazyRoute(<UiTunerPage />)} />
        <Route path="local-tasks" element={lazyRoute(<LocalTasksPage />)} />
        <Route path="codex-control" element={lazyRoute(<CodexControlPage />)} />
        <Route element={<ComputeWorkspaceLayout />}>
          <Route path="compute-settlement" element={lazyRoute(<ComputeSettlementPage />)} />
          <Route path="my-compute-settlement" element={lazyRoute(<MyComputeSettlementPage />)} />
          <Route path="compute-supply" element={lazyRoute(<ComputeSupplyPage />)} />
          <Route path="compute-activation" element={lazyRoute(<ComputeActivationAdminPage />)} />
          <Route path="compute-offers" element={lazyRoute(<ComputeOfferAdminPage />)} />
          <Route path="compute-reference-curves" element={lazyRoute(<ComputeReferenceCurvePage />)} />
          <Route path="compute-external-pools" element={lazyRoute(<ComputeExternalPoolsPage />)} />
          <Route path="compute-market" element={lazyRoute(<ComputeMarketPage />)} />
          <Route path="compute-reviews" element={lazyRoute(<ComputeConsumerReviewPage />)} />
          <Route path="compute-execution" element={lazyRoute(<ComputeExecutionPage />)} />
          <Route path="compute-observations" element={lazyRoute(<ComputePlatformObservationPage />)} />
          <Route path="compute-verification" element={lazyRoute(<ComputeVerificationPage />)} />
          <Route path="compute-receipts" element={lazyRoute(<ComputeExecutionReceiptPage />)} />
          <Route path="compute-finalization" element={lazyRoute(<ComputeAttemptFinalizationPage />)} />
          <Route path="compute-settlement-issuance" element={lazyRoute(<ComputeSettlementIssuancePage />)} />
          <Route path="compute-challenges" element={lazyRoute(<ComputeSettlementChallengePage />)} />
          <Route path="compute-challenge-resolution" element={lazyRoute(<ComputeSettlementChallengeResolutionPage />)} />
          <Route path="compute-corrections" element={lazyRoute(<ComputeSettlementCorrectionPage />)} />
          <Route path="node" element={lazyRoute(<NodePage />)} />
        </Route>
        <Route path="voice" element={lazyRoute(<VoicePage />)} />
        <Route path="doctor" element={lazyRoute(<DoctorPage />)} />
        <Route path="node/public-dev-smoke" element={lazyRoute(<PublicDevSmokePage />)} />
      </Route>
    </Routes>
  )
}
