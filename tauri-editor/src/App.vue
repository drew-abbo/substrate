<script setup lang="ts">
import TitleBar       from './components/TitleBar.vue'
import NodeGraph      from './components/NodeGraph.vue'
import FloatingOutput from './components/FloatingOutput.vue'
import MonitorScreen  from './components/MonitorScreen.vue'

const isOutputMode = new URLSearchParams(window.location.search).get('view') === 'output'

// The output window's webview must be see-through where the video shows —
// the engine renders onto a wgpu surface behind it (index.html paints the
// body opaque for the editor).
if (isOutputMode) document.body.style.background = 'transparent'
</script>

<template>
  <!-- ── Detached output window ── -->
  <div v-if="isOutputMode" class="output-mode">
    <TitleBar hide-menu />
    <MonitorScreen surface />
  </div>

  <!-- ── Main editor ── -->
  <template v-else>
    <div class="app">
      <TitleBar>
        <template #menu>
          <button class="tb-menu-btn">File</button>
          <button class="tb-menu-btn">Edit</button>
          <button class="tb-menu-btn">View</button>
        </template>
      </TitleBar>

      <div class="main-area">
        <NodeGraph ref="graphRef" class="graph-fill" />
      </div>

      <FloatingOutput />
    </div>
  </template>
</template>

<style>
@import './styles/theme.css';
@import './styles/globals.css';
@import './styles/nodes.css';

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body { width: 100%; height: 100%; overflow: hidden; }

/* ── Output-only window (detached) ──
   Transparent so the wgpu surface rendering behind the webview shows
   through; the video letterboxes into the MonitorScreen area. */
.output-mode {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: transparent;
  font-family: var(--font-ui);
  font-size: 13px;
  color: var(--text-1);
}

/* ── Main editor ── */
.app {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
  color: var(--text-1);
  font-family: var(--font-ui);
  font-size: 13px;
  overflow: hidden;
  user-select: none;
}

.main-area {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.graph-fill {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
</style>
