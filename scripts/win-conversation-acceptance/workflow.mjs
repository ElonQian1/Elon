import { releaseReady, safeCode } from './control.mjs'

// update.intent is checkpointed before the mutation; interrupted or ambiguous
// updates reconcile the exact running release instead of submitting again.
export async function runWorkflow({ state, control, save, read, delay = ms => new Promise(resolve => setTimeout(resolve, ms)) }) {
  const checkpoint = async stage => { state.stage = stage; state.updated_at = new Date().toISOString(); await save(state) }
  try {
    state.status = 'running'; delete state.error; delete state.read; delete state.progress; delete state.final_release
    delete state.desktop_release; delete state.desktop_process_ids
    state.workflow_complete = false
    await checkpoint('connecting')
    const connected = await control.connect(state.base, { waitOnly: Boolean(state.update && state.update.phase !== 'verified') })
    state.base = connected.base
    state.initial_release ??= connected.status.release_identity
    await checkpoint('connected')
    if (state.update?.phase === 'verified' && !releaseReady(connected.status, state.target_release)) throw Error('win_release_drift')
    if (releaseReady(connected.status, state.target_release)) {
      state.update = { ...state.update, phase: 'verified', mode: state.update?.mode || 'already_current' }
    } else {
      if (!state.update) {
        state.update = { phase: 'intent', mode: 'update_requested' }
        await checkpoint('updating')
        try {
          state.update.action = await control.submit('update_and_restart', state.target_release, state.run_id)
          state.update.phase = 'submitted'
          await checkpoint('updating')
        } catch (error) {
          state.update.error = safeCode(error)
          await checkpoint('update_reply_uncertain')
        }
      }
      if (state.update.action && state.update.phase !== 'scheduled') {
        try {
          state.update.action = await control.waitAction(state.update.action)
          state.update.phase = 'scheduled'
          await checkpoint('reconnecting')
        } catch (error) {
          if (['win_action_failed', 'win_action_rejected', 'win_action_expired', 'win_action_host_unavailable'].includes(safeCode(error))) throw error
          state.update.error = safeCode(error)
          await checkpoint('reconnecting')
        }
      }
      await control.waitRelease(state.target_release)
      state.update.phase = 'verified'
    }
    await checkpoint('release_verified')
    if (state.update_only) {
      const final = await control.status()
      if (!releaseReady(final, state.target_release)) throw Error('win_release_drift')
      state.final_release = final.release_identity
      state.desktop_release = final.desktop_release_identity
      state.desktop_process_ids = final.desktop_process_ids
      state.workflow_complete = true
      state.status = 'passed'
      await checkpoint('finished')
      return state
    }
    // These actions are reversible; a resumed run may repeat them. Page cursors
    // are deliberately discarded after an interruption or host restart.
    state.controls = []
    for (const kind of ['reload_page', 'navigate', 'capture_state']) {
      await checkpoint(kind)
      const action = await control.waitAction(await control.submit(kind, undefined, state.run_id))
      state.controls.push(action)
      await checkpoint(kind)
      if (kind === 'reload_page') await delay(1500)
      if (kind === 'navigate' && action.route !== '/ai') throw Error('win_navigation_unverified')
    }
    await checkpoint('reading')
    state.read = await read(async progress => { state.progress = progress; await checkpoint('reading') }, state.base)
    // Prevent a rolling update during a long read from being accepted as the
    // original artifact. The reader also pins its own host/snapshot identity.
    const final = await control.status()
    if (!releaseReady(final, state.target_release)) throw Error('win_release_drift')
    state.final_release = final.release_identity
    state.workflow_complete = true
    state.status = state.read.content_complete ? 'passed' : 'partial'
    await checkpoint('finished')
  } catch (error) {
    state.error = safeCode(error)
    state.status = ['login_required', 'http_401', 'http_403', 'auth_cooldown'].includes(state.error) ? 'user_action_required' : 'failed'
    await checkpoint(state.stage)
  }
  return state
}
