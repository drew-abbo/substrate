import { onMounted, onUnmounted, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * Binary layout of the `get_frame` response (see commands/frame.rs):
 * little-endian width u32, height u32, srcWidth u32, srcHeight u32,
 * generation u64, then RGBA pixels. The src dimensions are the engine output
 * resolution before preview downscaling.
 */
const HEADER_BYTES = 24

/**
 * Event-driven frame display: the Rust bridge emits "frame-ready" each time
 * it writes a new preview frame; we fetch + decode immediately, then blit on
 * the next animation frame. This removes the compounding of rAF interval
 * plus IPC round-trip that made polling choppy.
 */
function formatFps(num: number, den: number): string {
  const f = num / den
  if (Math.abs(f - 23.976) < 0.01) return '23.976'
  if (Math.abs(f - 29.97)  < 0.01) return '29.97'
  if (Math.abs(f - 59.94)  < 0.01) return '59.94'
  return den === 1 ? `${num}` : f.toFixed(3).replace(/\.?0+$/, '')
}

export function useFrameStream(canvas: Ref<HTMLCanvasElement | null>) {
  const hasFrame   = ref(false)
  const resolution = ref('-- × --')
  const fps        = ref('--')
  let running      = false
  let fetchLock    = false
  let rafHandle    = 0
  let unlisten:    UnlistenFn | null = null
  let unlistenFps: UnlistenFn | null = null
  let observer:    ResizeObserver | null = null

  async function reportSize() {
    const rect = canvas.value?.getBoundingClientRect()
    if (!rect || rect.width < 2 || rect.height < 2) return
    const dpr = window.devicePixelRatio || 1
    try {
      await invoke('set_preview_size', {
        width:  Math.round(rect.width * dpr),
        height: Math.round(rect.height * dpr),
      })
    } catch {
      // engine not connected yet
    }
  }

  async function onFrameReady() {
    if (!running || fetchLock) return
    fetchLock = true
    try {
      const buf = await invoke<ArrayBuffer>('get_frame')
      if (buf instanceof ArrayBuffer && buf.byteLength > HEADER_BYTES) {
        const view      = new DataView(buf)
        const width     = view.getUint32(0,  true)
        const height    = view.getUint32(4,  true)
        const srcWidth  = view.getUint32(8,  true)
        const srcHeight = view.getUint32(12, true)
        const pixels    = new Uint8ClampedArray(buf, HEADER_BYTES)
        if (pixels.length === width * height * 4) {
          // Copy into a new buffer owned by ImageData before the ArrayBuffer is GC'd.
          const imageData = new ImageData(new Uint8ClampedArray(pixels), width, height)
          hasFrame.value   = true
          resolution.value = `${srcWidth} × ${srcHeight}`
          // Blit on the next animation frame so it hits a vsync boundary.
          cancelAnimationFrame(rafHandle)
          rafHandle = requestAnimationFrame(() => {
            const ctx = canvas.value?.getContext('2d')
            if (!ctx || !canvas.value) return
            if (canvas.value.width  !== width)  canvas.value.width  = width
            if (canvas.value.height !== height) canvas.value.height = height
            ctx.putImageData(imageData, 0, 0)
          })
        }
      }
    } catch {
      // engine not connected yet
    } finally {
      fetchLock = false
    }
  }

  onMounted(async () => {
    running = true
    unlisten    = await listen('frame-ready', onFrameReady)
    unlistenFps = await listen<{ num: number; den: number }>('fps-changed', ({ payload }) => {
      fps.value = formatFps(payload.num, payload.den)
    })
    if (canvas.value) {
      observer = new ResizeObserver(() => { reportSize() })
      observer.observe(canvas.value)
    }
    reportSize()
  })
  onUnmounted(() => {
    running = false
    cancelAnimationFrame(rafHandle)
    unlisten?.()
    unlistenFps?.()
    observer?.disconnect()
  })

  return { hasFrame, resolution, fps }
}
