import { onMounted, onUnmounted, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

/** How often the status bar refreshes resolution / liveness. */
const INFO_POLL_MS = 500
/** GPU may still be initialising right after app launch. */
const ATTACH_RETRIES = 10
const ATTACH_RETRY_MS = 300

interface OutputInfo {
  width:  number
  height: number
  active: boolean
}

/**
 * Direct GPU output: attaches a wgpu surface to this window so the engine
 * bridge presents frames straight to it — no readback, no IPC, no canvas.
 * The given element marks where the video letterboxes; everything rendered
 * by the webview must be transparent there so the surface shows through.
 */
function formatFps(num: number, den: number): string {
  const f = num / den
  if (Math.abs(f - 23.976) < 0.01) return '23.976'
  if (Math.abs(f - 29.97)  < 0.01) return '29.97'
  if (Math.abs(f - 59.94)  < 0.01) return '59.94'
  return den === 1 ? `${num}` : f.toFixed(3).replace(/\.?0+$/, '')
}

export function useSurfaceOutput(el: Ref<HTMLElement | null>) {
  const hasFrame   = ref(false)
  const resolution = ref('-- × --')
  const fps        = ref('--')
  let observer:    ResizeObserver | null = null
  let unlistenFps: UnlistenFn | null = null
  let infoTimer = 0
  let attached  = false

  async function reportRect() {
    const rect = el.value?.getBoundingClientRect()
    if (!rect || rect.width < 1 || rect.height < 1) return
    const dpr = window.devicePixelRatio || 1
    try {
      await invoke('set_output_rect', {
        x:      rect.x * dpr,
        y:      rect.y * dpr,
        width:  rect.width * dpr,
        height: rect.height * dpr,
      })
    } catch {
      // bridge not ready yet; the resize observer will report again
    }
  }

  async function attach() {
    const label = getCurrentWebviewWindow().label
    for (let attempt = 0; attempt < ATTACH_RETRIES; attempt++) {
      try {
        await invoke('attach_output_surface', { windowLabel: label })
        attached = true
        return
      } catch {
        await new Promise(resolve => setTimeout(resolve, ATTACH_RETRY_MS))
      }
    }
    console.warn('Could not attach output surface; no video will show')
  }

  async function pollInfo() {
    try {
      const info = await invoke<OutputInfo>('get_output_info')
      hasFrame.value = info.active && info.width > 0
      if (info.width > 0) resolution.value = `${info.width} × ${info.height}`
    } catch {
      // ignore; retried on next tick
    }
  }

  onMounted(async () => {
    await attach()
    await reportRect()
    if (el.value) {
      observer = new ResizeObserver(() => { reportRect() })
      observer.observe(el.value)
    }
    window.addEventListener('resize', reportRect)
    infoTimer = window.setInterval(pollInfo, INFO_POLL_MS)
    pollInfo()
    unlistenFps = await listen<{ num: number; den: number }>('fps-changed', ({ payload }) => {
      fps.value = formatFps(payload.num, payload.den)
    })
  })
  onUnmounted(() => {
    observer?.disconnect()
    window.removeEventListener('resize', reportRect)
    window.clearInterval(infoTimer)
    unlistenFps?.()
    if (attached) invoke('detach_output_surface').catch(() => {})
  })

  return { hasFrame, resolution, fps }
}
