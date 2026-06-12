<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const win = getCurrentWindow()

function minimize() { win.minimize() }
function toggleMaximize() { win.toggleMaximize() }
function close() { win.close() }
</script>

<template>
  <header class="title-bar" data-tauri-drag-region>
    <div class="left">
      <div class="logo-badge">
        <img src="/substrate-logo.png" alt="Substrate" class="logo" draggable="false" />
      </div>
      <span class="app-name">Substrate</span>
    </div>

    <div class="drag-fill" data-tauri-drag-region />

    <div class="window-controls">
      <button class="wc-btn" @click="minimize" title="Minimize">
        <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button class="wc-btn" @click="toggleMaximize" title="Maximize">
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor"/></svg>
      </button>
      <button class="wc-btn close" @click="close" title="Close">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1.2"/>
          <line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.title-bar {
  display: flex;
  align-items: center;
  height: 36px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}

.left {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 12px;
  flex-shrink: 0;
}

.logo-badge {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  background: #fff;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.logo { width: 16px; height: 16px; object-fit: contain; pointer-events: none; }

.app-name {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--text-2);
}

.drag-fill { flex: 1; height: 100%; }

.window-controls { display: flex; align-items: stretch; height: 36px; }

.wc-btn {
  width: 44px;
  height: 100%;
  background: transparent;
  border: none;
  color: var(--text-3);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}
.wc-btn:hover { background: var(--bg-elevated); color: var(--text-1); }
.wc-btn.close:hover { background: var(--danger); color: #fff; }
</style>
