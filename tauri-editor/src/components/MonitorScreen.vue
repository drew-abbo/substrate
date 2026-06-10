<script setup lang="ts">
import { ref } from 'vue'
import { useFrameStream } from '../composables/frameStream'
import { useSurfaceOutput } from '../composables/surfaceOutput'

/**
 * `surface` switches from canvas polling (floating preview panel) to direct
 * GPU rendering: the backend letterboxes video into this element's rect on a
 * wgpu surface behind the webview, so the background here must stay
 * transparent for it to show through.
 */
const props = defineProps<{ surface?: boolean }>()

const canvas = ref<HTMLCanvasElement | null>(null)
const screen = ref<HTMLElement | null>(null)

const { hasFrame, resolution } = props.surface
  ? useSurfaceOutput(screen)
  : useFrameStream(canvas)
</script>

<template>
  <div ref="screen" class="monitor-screen" :class="{ 'surface-mode': surface }">
    <canvas v-if="!surface" ref="canvas" class="frame-canvas" :class="{ visible: hasFrame }" />

    <div class="no-signal" v-if="!hasFrame">
      <div class="ns-label">NO OUTPUT</div>
      <div class="ns-hint">Connect an Output node to preview</div>
    </div>

    <span class="mark tl" /><span class="mark tr" />
    <span class="mark bl" /><span class="mark br" />

    <div class="scanlines" />
    <div class="vignette"  />

    <div class="status-bar">
      <span class="stat">OUTPUT</span>
      <span class="stat mono">{{ resolution }}</span>
      <span class="stat mono" :class="{ live: hasFrame }">
        {{ hasFrame ? '● LIVE' : '○ IDLE' }}
      </span>
    </div>
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
</style>
