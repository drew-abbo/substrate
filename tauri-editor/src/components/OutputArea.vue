<script setup lang="ts">
import { ref } from 'vue'
import { useFrameStream } from '../composables/frameStream'

const canvas = ref<HTMLCanvasElement | null>(null)
const { hasFrame, resolution } = useFrameStream(canvas)
</script>

<template>
  <div class="output-wrap">
    <div class="monitor-bezel">
      <div class="screen">
        <canvas ref="canvas" class="frame-canvas" :class="{ visible: hasFrame }" />

        <div class="no-signal" v-if="!hasFrame">
          <div class="ns-label">NO OUTPUT</div>
          <div class="ns-hint">Connect an Output node to preview</div>
        </div>

        <span class="mark tl" /><span class="mark tr" />
        <span class="mark bl" /><span class="mark br" />

        <div class="scanlines" />
        <div class="vignette" />

        <div class="status-bar">
          <span class="stat">OUTPUT</span>
          <span class="stat mono">{{ resolution }}</span>
          <span class="stat mono">{{ hasFrame ? '● LIVE' : '○ IDLE' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.output-wrap {
  width: 100%;
  height: 100%;
  background: var(--bg-app);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 14px 20px 12px;
}

.monitor-bezel {
  width: 100%;
  max-width: calc((100vh - 36px - 260px) * (16 / 9));
  aspect-ratio: 16 / 9;
  background: var(--bg-app);
  border: 1px solid var(--border-default);
  border-radius: 3px;
  box-shadow:
    0 0 0 2px var(--bg-screen),
    0 8px 32px rgba(0, 0, 0, 0.9),
    inset 0 1px 0 rgba(255, 255, 255, 0.03);
  overflow: hidden;
  position: relative;
  flex-shrink: 0;
}

.screen {
  position: absolute;
  inset: 0;
  background: var(--bg-screen);
  overflow: hidden;
}

/* OutputArea-specific overrides for globals.css status-bar */
.status-bar {
  padding: 0 12px;
}
</style>
