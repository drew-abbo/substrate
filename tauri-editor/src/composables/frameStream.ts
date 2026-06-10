import { onMounted, onUnmounted, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/**
 * Binary layout of the `get_frame` response (see commands/frame.rs):
 * little-endian width u32, height u32, srcWidth u32, srcHeight u32,
 * generation u64, then RGBA pixels. The src dimensions are the engine output
 * resolution before preview downscaling.
 */
const HEADER_BYTES = 24

/**
 * Polls the backend for engine frames on every animation frame and blits
 * them onto the given canvas. Reports the canvas's physical pixel size so
 * the backend downscales frames to preview size before the readback + IPC.
 */
export function useFrameStream(canvas: Ref<HTMLCanvasElement | null>) {
  const hasFrame   = ref(false)
  const resolution = ref('-- × --')
  let rafHandle = 0
  let running   = false
  let observer: ResizeObserver | null = null

  async function reportSize() {
    const rect = canvas.value?.getBoundingClientRect()
    // Skip while hidden (display:none) so we don't reset the preview size.
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

  async function renderLoop() {
    if (!running) return
    const ctx = canvas.value?.getContext('2d')
    if (ctx && canvas.value) {
      try {
        const buf = await invoke<ArrayBuffer>('get_frame')
        if (buf instanceof ArrayBuffer && buf.byteLength > HEADER_BYTES) {
          const view      = new DataView(buf)
          const width     = view.getUint32(0,  true)
          const height    = view.getUint32(4,  true)
          const srcWidth  = view.getUint32(8,  true)
          const srcHeight = view.getUint32(12, true)
          const pixels = new Uint8ClampedArray(buf, HEADER_BYTES)
          if (pixels.length === width * height * 4) {
            if (canvas.value.width  !== width)  canvas.value.width  = width
            if (canvas.value.height !== height) canvas.value.height = height
            ctx.putImageData(new ImageData(pixels, width, height), 0, 0)
            hasFrame.value   = true
            resolution.value = `${srcWidth} × ${srcHeight}`
          }
        }
      } catch {
        // engine not connected yet
      }
    }
    rafHandle = requestAnimationFrame(renderLoop)
  }

  onMounted(() => {
    running = true
    rafHandle = requestAnimationFrame(renderLoop)
    if (canvas.value) {
      observer = new ResizeObserver(() => { reportSize() })
      observer.observe(canvas.value)
    }
    reportSize()
  })
  onUnmounted(() => {
    running = false
    cancelAnimationFrame(rafHandle)
    observer?.disconnect()
  })

  return { hasFrame, resolution }
}
