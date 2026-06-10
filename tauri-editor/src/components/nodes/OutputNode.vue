<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'

defineProps<{ selected?: boolean }>()
</script>

<template>
  <div class="output-node" :class="{ selected }">
    <!-- Input handle centered on the screen body -->
    <Handle
      id="out-in-graph"
      type="target"
      :position="Position.Left"
      class="port-handle video"
      style="top: 56px"
    />

    <div class="output-header">
      <svg class="output-icon" viewBox="0 0 16 16" fill="none"
           stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
        <path d="M1 2h14v9H1zM5 11v3M11 11v3M3 14h10"/>
      </svg>
      <span class="output-title">OUTPUT</span>
    </div>

    <div class="output-body">
      <div class="output-screen">
        <svg viewBox="0 0 16 16" fill="none" class="screen-icon">
          <polygon points="5,3 13,8 5,13" fill="currentColor"/>
        </svg>
      </div>
    </div>
  </div>
</template>

<style scoped>
.output-node {
  width: 160px;
  background: var(--bg-node);
  border: 1px solid var(--border-default);
  border-left: 3px solid var(--cat-output);
  border-radius: 6px;
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.55);
  cursor: default;
  transition: border-color 0.12s, box-shadow 0.12s;
}

.output-node.selected {
  border-color: var(--accent);
  border-left-color: var(--cat-output);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.65), var(--accent-glow);
}

.output-header {
  height: 30px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 10px 0 8px;
  border-bottom: 1px solid var(--border-subtle);
}

.output-icon {
  width: 13px;
  height: 13px;
  color: var(--cat-output);
  flex-shrink: 0;
  opacity: 0.9;
}

.output-title {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.07em;
  color: var(--text-1);
}

.output-body {
  height: 52px;
  padding: 8px 10px;
}

.output-screen {
  width: 100%;
  height: 100%;
  background: var(--bg-app);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.screen-icon {
  width: 16px;
  height: 16px;
  color: var(--cat-output);
  opacity: 0.35;
}
</style>
