import { useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import { commitSourcePreview, loadSourcePreview } from './sourcePreviewApi'
import { findSourceNode, sourcePatchValue, updateSourceNode } from './sourcePreviewTree'
import type { PendingSourceNodePatch, SourcePreviewDocument, SourcePreviewPatch, SourcePreviewSaveState } from './types'
import { useSourceRenderer } from './useSourceRenderer'

interface PreviewSnapshot { document: SourcePreviewDocument; pending: Record<string, PendingSourceNodePatch> }
interface PreviewHistory { past: PreviewSnapshot[]; future: PreviewSnapshot[] }
interface EditorState { document: SourcePreviewDocument | null; pending: Record<string, PendingSourceNodePatch>; history: PreviewHistory }
type EditorAction =
  | { type: 'load'; document: SourcePreviewDocument }
  | { type: 'apply'; patch: SourcePreviewPatch }
  | { type: 'undo' }
  | { type: 'redo' }

const EMPTY_EDITOR: EditorState = { document: null, pending: {}, history: { past: [], future: [] } }

function editorReducer(state: EditorState, action: EditorAction): EditorState {
  if (action.type === 'load') return { document: action.document, pending: {}, history: { past: [], future: [] } }
  if (action.type === 'undo') {
    const previous = state.history.past[state.history.past.length - 1]
    if (!previous || !state.document) return state
    return { document: previous.document, pending: previous.pending, history: { past: state.history.past.slice(0, -1), future: [{ document: state.document, pending: state.pending }, ...state.history.future] } }
  }
  if (action.type === 'redo') {
    const next = state.history.future[0]
    if (!next || !state.document) return state
    return { document: next.document, pending: next.pending, history: { past: [...state.history.past, { document: state.document, pending: state.pending }], future: state.history.future.slice(1) } }
  }
  if (!state.document) return state
  const node = findSourceNode(state.document.root, action.patch.nodeKey)
  if (!node) return state
  const document = { ...state.document, root: updateSourceNode(state.document.root, action.patch) }
  const pending = {
    ...state.pending,
    [node.key]: {
      nodeKey: node.key,
      startTagStart: node.source.startTagStart,
      startTagEnd: node.source.startTagEnd,
      changes: { ...state.pending[node.key]?.changes, [action.patch.property]: sourcePatchValue(action.patch.property, action.patch.value) },
    },
  }
  return { document, pending, history: { past: [...state.history.past.slice(-49), { document: state.document, pending: state.pending }], future: [] } }
}

export function useSourcePreviewSession(initialProjectRoot: string) {
  const [projectRoot, setProjectRoot] = useState(initialProjectRoot)
  const [editor, dispatch] = useReducer(editorReducer, EMPTY_EDITOR)
  const [selectedKey, setSelectedKey] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [saveState, setSaveState] = useState<SourcePreviewSaveState>('preview')
  const renderer = useSourceRenderer()

  useEffect(() => {
    if (initialProjectRoot && !projectRoot) setProjectRoot(initialProjectRoot)
  }, [initialProjectRoot, projectRoot])

  const load = useCallback(async (layoutFile?: string) => {
    const root = projectRoot.trim()
    if (!root) { setError('请先选择或输入本机 Android 项目目录'); return }
    setLoading(true); setError('')
    try {
      const rendererRefresh = renderer.refresh(root)
      const next = await loadSourcePreview(root, layoutFile)
      dispatch({ type: 'load', document: next })
      setSelectedKey(next.root.key)
      setSaveState('preview')
      window.localStorage.setItem('elon.uiTuner.sourceProjectRoot', root)
      await rendererRefresh
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    } finally { setLoading(false) }
  }, [projectRoot, renderer.refresh])

  const apply = useCallback((patch: SourcePreviewPatch) => { renderer.beginLocalDraft(); dispatch({ type: 'apply', patch }); setSaveState('preview') }, [renderer.beginLocalDraft])
  const undo = useCallback(() => { dispatch({ type: 'undo' }); setSaveState('preview') }, [])
  const redo = useCallback(() => { dispatch({ type: 'redo' }); setSaveState('preview') }, [])

  const commit = useCallback(async () => {
    const { document, pending } = editor
    if (!document || Object.keys(pending).length === 0) return
    setSaveState('saving'); setError('')
    try {
      let revision = document.sourceRevision
      const patches = Object.values(pending).sort((a, b) => b.startTagStart - a.startTagStart)
      for (const patch of patches) {
        const result = await commitSourcePreview({
          projectRoot: document.projectRoot,
          layoutFile: document.selectedLayout,
          sourceRevision: revision,
          ...patch,
        })
        revision = result.sourceRevision
      }
      const reloaded = await loadSourcePreview(document.projectRoot, document.selectedLayout)
      dispatch({ type: 'load', document: reloaded }); setSaveState('saved')
      await renderer.rerender(document.projectRoot)
    } catch (reason) {
      setSaveState('error'); setError(reason instanceof Error ? reason.message : String(reason))
    }
  }, [editor, renderer.rerender])

  const selected = useMemo(() => findSourceNode(editor.document?.root ?? null, selectedKey), [editor.document, selectedKey])
  return { projectRoot, setProjectRoot, document: editor.document, selected, selectedKey, setSelectedKey, pending: editor.pending, history: editor.history, loading, error, saveState, load, apply, undo, redo, commit, renderer }
}
