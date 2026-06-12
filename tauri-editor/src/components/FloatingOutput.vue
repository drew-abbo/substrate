<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import MonitorScreen from './MonitorScreen.vue'

// ── Panel dimensions ─────────────────────────────────────
const MIN_W = 220
const MAX_W = 1200
// header(30) + body-pad(6+6) + bezel follows aspect-ratio from CSS
// height formula: 42 + (width - 12) * (9/16)
function computeHeight(w: number) {
  return 42 + (w - 12) * (9 / 16)
}

const panelRef  = ref<HTMLElement | null>(null)
const width     = ref(360)
const pos       = ref({ x: 20, y: 56 })
const minimized  = ref(false)
const isDetached = ref(false)

// ── Drag ─────────────────────────────────────────────────
let dragging    = false
let dragStart   = { mx: 0, my: 0, px: 0, py: 0 }

function startDrag(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('button, .resize-handle')) return
  dragging  = true
  dragStart = { mx: e.clientX, my: e.clientY, px: pos.value.x, py: pos.value.y }
  e.preventDefault()
}

// ── Resize ───────────────────────────────────────────────
type Corner = 'tl' | 'tr' | 'bl' | 'br'
let resizing: Corner | null = null
let resizeStart = { mx: 0, w: 0, px: 0, py: 0 }

function startResize(e: MouseEvent, corner: Corner) {
  resizing    = corner
  resizeStart = {
    mx: e.clientX,
    w:  width.value,
    px: pos.value.x,
    py: pos.value.y,
  }
  e.preventDefault()
  e.stopPropagation()
}

// ── Shared move handler ───────────────────────────────────
function onMouseMove(e: MouseEvent) {
  if (dragging) {
    const dx = e.clientX - dragStart.mx
    const dy = e.clientY - dragStart.my
    pos.value = clampPos(dragStart.px + dx, dragStart.py + dy, width.value)
    return
  }

  if (resizing) {
    const dx = e.clientX - resizeStart.mx
    let newW: number
    let newX = resizeStart.px
    let newY = resizeStart.py

    // left-anchored corners shrink on left drag
    if (resizing === 'br' || resizing === 'tr') {
      newW = resizeStart.w + dx
    } else {
      newW = resizeStart.w - dx
      newX = resizeStart.px + resizeStart.w - newW
    }

    newW = Math.max(MIN_W, Math.min(MAX_W, newW))

    // top corners: keep bottom edge fixed
    if (resizing === 'tl' || resizing === 'tr') {
      const oldH = computeHeight(resizeStart.w)
      const newH = computeHeight(newW)
      newY = resizeStart.py + (oldH - newH)
    }

    // Re-clamp X for left-side resize (newX may have moved left past boundary)
    newX = Math.max(4, newX)
    newY = Math.max(36, newY)

    width.value = newW
    pos.value   = { x: newX, y: newY }
  }
}

function stopAll() {
  dragging = false
  resizing = null
}

function clampPos(x: number, y: number, w: number) {
  const h    = computeHeight(w)
  const maxX = document.documentElement.clientWidth  - w - 4
  const maxY = document.documentElement.clientHeight - h - 4
  return { x: Math.max(4, Math.min(x, maxX)), y: Math.max(36, Math.min(y, maxY)) }
}

function onWindowResize() {
  pos.value = clampPos(pos.value.x, pos.value.y, width.value)
}

// ── Detach ────────────────────────────────────────────────
async function detach() {
  try {
    const win = new WebviewWindow('output-monitor', {
      url:        'index.html?view=output',
      title:      'Output Monitor — Bio Visualizer',
      width:      960,
      height:     540,
      minWidth:   480,
      minHeight:  270,
      resizable:  true,
      decorations: false,
      // The webview must be see-through: the engine presents video onto a
      // wgpu surface attached behind it (see composables/surfaceOutput.ts).
      transparent: true,
    })
    win.once('tauri://created',   () => { isDetached.value = true;  minimized.value = true  })
    win.once('tauri://destroyed', () => { isDetached.value = false; minimized.value = false })
  } catch (err) {
    console.warn('Could not open output window:', err)
  }
}

// ── Lifecycle ─────────────────────────────────────────────
onMounted(() => {
  const el = panelRef.value
  if (el) {
    pos.value = {
      x: document.documentElement.clientWidth  - width.value - 24,
      y: 36 + 20,
    }
  }
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup',   stopAll)
  window.addEventListener('resize',    onWindowResize)
})
onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup',   stopAll)
  window.removeEventListener('resize',    onWindowResize)
})
</script>

<template>
  <div
    ref="panelRef"
    class="floating-panel"
    :class="{ minimized }"
    :style="{ left: pos.x + 'px', top: pos.y + 'px', width: width + 'px' }"
  >
    <!-- Corner resize handles -->
    <template v-if="!minimized">
      <div class="resize-handle corner-tl" @mousedown.stop="startResize($event, 'tl')" />
      <div class="resize-handle corner-tr" @mousedown.stop="startResize($event, 'tr')" />
      <div class="resize-handle corner-bl" @mousedown.stop="startResize($event, 'bl')" />
      <div class="resize-handle corner-br" @mousedown.stop="startResize($event, 'br')" />
    </template>

    <!-- Header -->
    <div class="panel-header" @mousedown="startDrag">
      <svg class="drag-icon" viewBox="0 0 16 16" fill="currentColor">
        <rect x="3"  y="4"  width="2" height="2" rx="0.5"/>
        <rect x="7"  y="4"  width="2" height="2" rx="0.5"/>
        <rect x="11" y="4"  width="2" height="2" rx="0.5"/>
        <rect x="3"  y="8"  width="2" height="2" rx="0.5"/>
        <rect x="7"  y="8"  width="2" height="2" rx="0.5"/>
        <rect x="11" y="8"  width="2" height="2" rx="0.5"/>
        <rect x="3"  y="12" width="2" height="2" rx="0.5"/>
        <rect x="7"  y="12" width="2" height="2" rx="0.5"/>
        <rect x="11" y="12" width="2" height="2" rx="0.5"/>
      </svg>
      <span class="panel-title">{{ isDetached ? 'Output (detached)' : 'Output' }}</span>

      <div class="header-btns">
        <button class="hbtn" :title="isDetached ? 'Already detached' : 'Open in separate window'"
                :disabled="isDetached" @click="detach">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
            <rect x="1" y="4" width="8" height="8" rx="1"/>
            <path d="M6 1h7v7M8 6l5-5"/>
          </svg>
        </button>
        <button class="hbtn" :title="minimized ? 'Expand' : 'Minimize'" @click="minimized = !minimized">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
            <path v-if="!minimized" d="M2 7h10"/>
            <path v-else            d="M2 5h10M2 9h10"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Screen -->
    <div class="panel-body" v-show="!minimized && !isDetached">
      <div class="bezel">
        <MonitorScreen />
      </div>
    </div>

    <div class="detached-notice" v-if="isDetached && !minimized">
      Output open in separate window
    </div>
  </div>
</template>

<style scoped>
.floating-panel {
  position: fixed;
  z-index: 200;
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  box-shadow: var(--shadow-float);
  overflow: visible;
  user-select: none;
}

/* ── Corner resize handles ──────────────────────────────── */
.resize-handle {
  position: absolute;
  width: 18px;
  height: 18px;
  z-index: 10;
}
/* cursors */
.corner-br { bottom: -5px; right: -5px; cursor: nwse-resize; }
.corner-bl { bottom: -5px; left:  -5px; cursor: nesw-resize; }
.corner-tr { top:    -5px; right: -5px; cursor: nesw-resize; }
.corner-tl { top:    -5px; left:  -5px; cursor: nwse-resize; }

/* visual dot in each corner */
.resize-handle::after {
  content: '';
  position: absolute;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: rgba(0, 204, 168, 0.2);
  transition: background 0.15s, transform 0.15s;
}
.resize-handle:hover::after {
  background: rgba(0, 204, 168, 0.7);
  transform: scale(1.2);
}
.corner-br::after { bottom: 2px; right: 2px; }
.corner-bl::after { bottom: 2px; left:  2px; }
.corner-tr::after { top:    2px; right: 2px; }
.corner-tl::after { top:    2px; left:  2px; }

/* ── Header ─────────────────────────────────────────────── */
.panel-header {
  height: 30px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px 0 6px;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-subtle);
  border-radius: 6px 6px 0 0;
  cursor: grab;
}
.panel-header:active { cursor: grabbing; }

.drag-icon { width: 12px; height: 12px; color: var(--text-3); flex-shrink: 0; }

.panel-title {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--text-2);
  flex: 1;
}

.header-btns { display: flex; gap: 2px; align-items: center; }

.hbtn {
  width: 22px; height: 22px;
  background: transparent;
  border: none; border-radius: 3px;
  color: var(--text-3);
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  padding: 0;
  transition: background 0.1s, color 0.1s;
}
.hbtn:hover    { background: var(--bg-elevated); color: var(--text-1); }
.hbtn:disabled { opacity: 0.3; cursor: not-allowed; }
.hbtn svg      { width: 12px; height: 12px; }

/* ── Body / bezel ───────────────────────────────────────── */
.panel-body { padding: 6px; }

.bezel {
  aspect-ratio: 16 / 9;
  width: 100%;
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  overflow: hidden;
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.02);
}

/* ── Detached notice ───────────────────────────────────── */
.detached-notice {
  padding: 10px 12px;
  font-size: 10px;
  color: var(--text-3);
  letter-spacing: 0.04em;
  text-align: center;
}
</style>
