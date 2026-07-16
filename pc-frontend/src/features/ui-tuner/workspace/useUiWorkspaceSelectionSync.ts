import { useCallback, useMemo } from 'react'
import type { UiTunerElement } from '../types'
import {
  evidenceSelectionHint,
  findEvidenceSelection,
  type UiWorkspaceSelectionHint,
} from './uiWorkspaceSelection'

export function useUiWorkspaceSelectionSync(
  selected: UiTunerElement | null,
  elements: UiTunerElement[],
  onSelectEvidence: (id: string) => void,
) {
  const sourceHint = useMemo(() => evidenceSelectionHint(selected), [selected])
  const onSourceSelection = useCallback((hint: UiWorkspaceSelectionHint) => {
    const match = findEvidenceSelection(elements, hint)
    if (match) onSelectEvidence(match)
  }, [elements, onSelectEvidence])
  return { sourceHint, onSourceSelection }
}

