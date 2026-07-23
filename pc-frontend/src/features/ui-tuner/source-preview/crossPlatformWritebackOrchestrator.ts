import type { SourcePreviewNode } from './types'
import type { PwaDesignDraft } from './pwaDesignDraft'
import {
  applyDeterministicAndroidWriteback,
  applyDeterministicPwaWriteback,
  planPwaDesignWriteback,
  recordDeterministicWriteback,
  type PwaCrossPlatformWritebackResult,
} from './pwaDesignWriteback'
import {
  beginCrossPlatformWritebackReceipt,
  completeCrossPlatformWritebackReceipt,
  type CrossPlatformWritebackReceipt,
  type PlatformReceiptUpdate,
} from './crossPlatformWritebackReceipt'

export interface CrossPlatformDeterministicResult {
  draft: PwaDesignDraft
  result: PwaCrossPlatformWritebackResult
  receipt: CrossPlatformWritebackReceipt
  conflict: boolean
  plan: ReturnType<typeof planPwaDesignWriteback>
}

export async function executeCrossPlatformDeterministicWriteback(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  onReceipt?: (receipt: CrossPlatformWritebackReceipt) => void
}): Promise<CrossPlatformDeterministicResult> {
  const operationId = `draft:${input.draft.project.id}:${input.draft.revision}:${Date.now()}`
  const initialPlan = planPwaDesignWriteback(input.draft, input.root)
  const initialReceipt = await beginCrossPlatformWritebackReceipt({
    operationId,
    projectRoot: input.draft.project.workspaceIdentity,
    draftRevision: input.draft.revision,
    targetPlatforms: ['pwa', 'apk'],
  })
  input.onReceipt?.(initialReceipt)
  const android = await applyDeterministicAndroidWriteback({
    draft: input.draft,
    root: input.root,
    projectRoot: input.draft.project.workspaceIdentity,
    sourceRevision: input.draft.project.sourceRevision,
  })
  let latest = recordDeterministicWriteback(input.draft, {
    android,
    pwa: emptyPwaResult(),
  })
  if (android.applied) {
    latest = { ...latest, project: { ...latest.project, sourceRevision: android.sourceRevision } }
  }
  const pwa = await applyDeterministicPwaWriteback({
    draft: latest,
    root: input.root,
    projectRoot: latest.project.workspaceIdentity,
  })
  const result = { android, pwa }
  latest = recordDeterministicWriteback(latest, {
    android: { ...android, completed: [] },
    pwa,
  })
  const receipt = await completeCrossPlatformWritebackReceipt({
    receiptId: initialReceipt.receiptId,
    projectRoot: latest.project.workspaceIdentity,
    platformResults: {
      apk: deterministicPlatformUpdate(
        android.error ? 'FAILED' : platformNeedsCodex(initialPlan, 'android') ? 'AI_WRITING' : 'SAVED',
        android.changedFiles,
        Object.fromEntries(android.changedFiles.map((file) => [
          file,
          android.sourceHashes[file] || android.sourceRevision,
        ])),
        android.error,
      ),
      pwa: deterministicPlatformUpdate(
        pwa.error ? 'FAILED' : platformNeedsCodex(initialPlan, 'pwa') ? 'AI_WRITING' : 'SAVED',
        pwa.changedFiles,
        pwa.sourceRevisions,
        pwa.error,
      ),
    },
  })
  return {
    draft: latest,
    result,
    receipt,
    conflict: Boolean(android.error || pwa.error),
    plan: planPwaDesignWriteback(latest, input.root),
  }
}

function deterministicPlatformUpdate(
  status: 'AI_WRITING' | 'SAVED' | 'FAILED',
  changedFiles: string[],
  sourceRevisions: Record<string, string>,
  error?: string,
): PlatformReceiptUpdate {
  const cleanRevisions = Object.fromEntries(
    Object.entries(sourceRevisions).filter(([file]) => Boolean(file)),
  )
  return {
    status,
    method: 'DETERMINISTIC',
    changedFiles,
    sourceRevisions: cleanRevisions,
    error,
  }
}

function platformNeedsCodex(
  plan: ReturnType<typeof planPwaDesignWriteback>,
  platform: 'pwa' | 'android',
) {
  return plan.codexChanges.some((change) => change.platform === platform)
}

function emptyPwaResult(): PwaCrossPlatformWritebackResult['pwa'] {
  return {
    applied: 0,
    changedFiles: [],
    sourceRevisions: {},
    sourceHashes: {},
    completed: [],
  }
}

export function pendingPlatformFiles(
  receipt: CrossPlatformWritebackReceipt,
  platform: 'pwa' | 'apk',
) {
  return receipt.platformResults[platform]?.changedFiles ?? []
}

export function mergedPlatformFiles(
  receipt: CrossPlatformWritebackReceipt,
  platform: 'pwa' | 'apk',
  additions: string[],
) {
  return [...new Set([...pendingPlatformFiles(receipt, platform), ...additions])].sort()
}
