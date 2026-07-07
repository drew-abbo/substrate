<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

defineProps<{ hideMenu?: boolean }>()

const win = getCurrentWindow()

function minimize() { win.minimize() }
function toggleMaximize() { win.toggleMaximize() }
function close() { win.close() }
</script>

<template>
  <header class="title-bar" data-tauri-drag-region>

    <!-- Left: logo + menu slot -->
    <div class="left">
      <div class="logo-badge">
        <img src="/substrate-logo.png" alt="Substrate" class="logo" draggable="false" />
      </div>

      <template v-if="!hideMenu">
        <div class="separator" />
        <nav class="menu">
          <slot name="menu" />
        </nav>
      </template>
    </div>

    <!-- Center: fill remaining space, drag region only -->
    <div class="drag-fill" data-tauri-drag-region />

    <!-- Right: optional tool area + window controls -->
    <div class="right">
      <div class="tools">
        <slot name="tools" />
      </div>

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
  gap: 0;
}

/* ── Left ─────────────────────────────────── */
.left {
  display: flex;
  align-items: center;
  gap: 6px;
  padding-left: 8px;
  flex-shrink: 0;
}

.logo-badge {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  background: #fff;
  overflow: hidden;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo {
  width: 16px;
  height: 16px;
  object-fit: contain;
  pointer-events: none;
}

.separator {
  width: 1px;
  height: 14px;
  background: var(--border-strong);
  margin: 0 4px;
  flex-shrink: 0;
}

.menu {
  display: flex;
  align-items: center;
  gap: 2px;
}

/* ── Center drag fill ─────────────────────── */
.drag-fill {
  flex: 1;
  height: 100%;
}

/* ── Right ────────────────────────────────── */
.right {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.tools {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-right: 4px;
}

/* ── Window controls ──────────────────────── */
.window-controls {
  display: flex;
  align-items: stretch;
  height: 36px;
}

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

.wc-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-1);
}

.wc-btn.close:hover {
  background: var(--danger);
  color: #fff;
}
</style>

<!-- Global styles for menu/tool buttons injected via slots -->
<style>
.tb-menu-btn {
  height: 24px;
  padding: 0 8px;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-2);
  font-size: 12px;
  font-family: var(--font-ui);
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s, color 0.1s;
}

.tb-menu-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-1);
}

.tb-menu-btn.active {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-1);
}

.tb-tool-btn {
  width: 26px;
  height: 26px;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-3);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}

.tb-tool-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-1);
}
</style>
