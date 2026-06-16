<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useFrameStream } from '../composables/frameStream'
import { useSurfaceOutput } from '../composables/surfaceOutput'
import { usePlayback } from '../composables/playback'

/**
 * `surface` switches from canvas polling (floating preview panel) to direct
 * GPU rendering: the backend letterboxes video into this element's rect on a
 * wgpu surface behind the webview, so the background here must stay
 * transparent for it to show through.
 */
const props = defineProps<{ surface?: boolean }>()

const canvas = ref<HTMLCanvasElement | null>(null)
const screen = ref<HTMLElement | null>(null)

const { hasFrame, fps } = props.surface
  ? useSurfaceOutput(screen)
  : useFrameStream(canvas)

// ── Playback controls (surface / detached window only) ────
const { isPlaying, togglePlay, syncState } = usePlayback()
const showControls = ref(false)
let hideTimer = 0

function onMouseMove() {
  if (!props.surface) return
  showControls.value = true
  window.clearTimeout(hideTimer)
  hideTimer = window.setTimeout(() => {
    showControls.value = false
  }, 2500)
}

onMounted(async () => {
  if (props.surface) {
    await syncState()
    hideTimer = window.setTimeout(() => {
      showControls.value = false
    }, 2500)
  }
})

onUnmounted(() => {
  window.clearTimeout(hideTimer)
})
</script>

<template>
  <div
    ref="screen"
    class="monitor-screen"
    :class="{ 'surface-mode': surface, 'cursor-hidden': surface && !showControls }"
    @mousemove="onMouseMove"
  >
    <canvas v-if="!surface" ref="canvas" class="frame-canvas" :class="{ visible: hasFrame }" />

    <div class="no-signal" v-if="!hasFrame">
      <div class="ns-label">NO OUTPUT</div>
      <div class="ns-hint">Connect an Output node to preview</div>
    </div>

    <!-- Controls overlay — surface / detached window only, auto-hides -->
    <Transition name="controls-fade">
      <div v-if="surface && showControls" class="controls-overlay">
        <button class="ctrl-btn" @click="togglePlay" :title="isPlaying ? 'Pause' : 'Play'">
          <svg v-if="isPlaying" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="2" width="3.5" height="12" rx="1"/>
            <rect x="9.5" y="2" width="3.5" height="12" rx="1"/>
          </svg>
          <svg v-else viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 2.5l10 5.5-10 5.5V2.5z"/>
          </svg>
        </button>
        <span class="ctrl-fps" v-if="hasFrame">{{ fps }} fps</span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.monitor-screen {
  position: relative;
  width: 100%;
  height: 100%;
  background: var(--bg-screen);
  overflow: hidden;
}

/* The wgpu surface behind the webview provides the background. */
.monitor-screen.surface-mode {
  background: transparent;
}

/* Hide cursor when controls have auto-hidden in surface mode */
.monitor-screen.cursor-hidden {
  cursor: none;
}

/* ── Surface-mode controls overlay ───────────────────────── */
.controls-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 52px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
  z-index: 10;
}

.ctrl-btn {
  width: 36px;
  height: 36px;
  background: rgba(255, 255, 255, 0.12);
  border: none;
  border-radius: 50%;
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s;
  flex-shrink: 0;
  padding: 0;
}
.ctrl-btn:hover { background: rgba(255, 255, 255, 0.22); }
.ctrl-btn svg   { width: 16px; height: 16px; }

.ctrl-fps {
  font-size: 11px;
  font-family: var(--font-mono);
  color: rgba(255, 255, 255, 0.6);
  white-space: nowrap;
}

/* Fade + slide transition */
.controls-fade-enter-active,
.controls-fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}
.controls-fade-enter-from,
.controls-fade-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
