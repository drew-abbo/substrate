import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Singleton — shared across all component instances
const manualEnabled = ref(false)
const manualFps = ref(30)

export const FPS_PRESETS = [24, 25, 30, 60, 120] as const

export function useFpsOverride() {
  async function setFps(fps: number) {
    const clamped = Math.round(Math.max(1, Math.min(999, fps)))
    try {
      await invoke('set_target_fps', { fps: clamped })
      manualFps.value = clamped
      manualEnabled.value = true
    } catch (err) {
      console.warn('set_target_fps failed:', err)
    }
  }

  async function clearFps() {
    try {
      await invoke('clear_target_fps')
      manualEnabled.value = false
    } catch (err) {
      console.warn('clear_target_fps failed:', err)
    }
  }

  async function toggle() {
    if (manualEnabled.value) await clearFps()
    else await setFps(manualFps.value)
  }

  return { manualEnabled, manualFps, setFps, clearFps, toggle }
}
