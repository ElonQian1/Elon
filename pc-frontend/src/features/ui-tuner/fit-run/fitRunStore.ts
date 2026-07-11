import { create } from 'zustand'
import type { TargetCurrentPair } from '../comparison/types'
import type { FitRunDocument } from './types'

interface FitRunBridgeState {
  pair: TargetCurrentPair | null
  run: FitRunDocument | null
  setPair: (pair: TargetCurrentPair | null) => void
  setRun: (run: FitRunDocument | null) => void
}

export const useFitRunStore = create<FitRunBridgeState>((set) => ({
  pair: null,
  run: null,
  setPair: (pair) => set({ pair }),
  setRun: (run) => set({ run }),
}))
