<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'

interface NodeDef {
  nodeType: string
  label: string
  category: string
}

const props = defineProps<{
  x: number
  y: number
  nodes: NodeDef[]
}>()

const emit = defineEmits<{
  select: [nodeType: string]
  close:  []
}>()

const searchRef = ref<HTMLInputElement | null>(null)
const search    = ref('')
const activeIdx = ref(0)

// ── Group by category ────────────────────────────────────
const sections = computed(() => {
  const q = search.value.toLowerCase()
  const map = new Map<string, NodeDef[]>()

  for (const node of props.nodes) {
    if (q && !node.label.toLowerCase().includes(q) && !node.category.toLowerCase().includes(q)) continue
    const arr = map.get(node.category) ?? []
    arr.push(node)
    map.set(node.category, arr)
  }

  return [...map.entries()].map(([title, nodes]) => ({ title, nodes }))
})

const flatFiltered = computed(() => sections.value.flatMap(s => s.nodes))

// ── Position: flip if near edge ──────────────────────────
const PICKER_W = 220
const PICKER_H = 320

const style = computed(() => {
  const vw = document.documentElement.clientWidth
  const vh = document.documentElement.clientHeight
  const x = props.x + PICKER_W > vw ? props.x - PICKER_W : props.x
  const y = props.y + PICKER_H > vh ? props.y - PICKER_H : props.y
  return { left: x + 'px', top: y + 'px' }
})

// ── Category accent colors ────────────────────────────────
const catColor: Record<string, string> = {
  sources:  'var(--cat-source)',
  audio:    'var(--cat-audio)',
  effects:  'var(--cat-effect)',
  analysis: 'var(--cat-analysis)',
  output:   'var(--cat-output)',
}
function pip(cat: string) { return catColor[cat.toLowerCase()] ?? 'var(--cat-utility)' }

// ── Keyboard navigation ───────────────────────────────────
function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape')     { emit('close'); return }
  if (e.key === 'ArrowDown')  { e.preventDefault(); activeIdx.value = Math.min(activeIdx.value + 1, flatFiltered.value.length - 1) }
  if (e.key === 'ArrowUp')    { e.preventDefault(); activeIdx.value = Math.max(activeIdx.value - 1, 0) }
  if (e.key === 'Enter')      { const node = flatFiltered.value[activeIdx.value]; if (node) emit('select', node.nodeType) }
}

// ── Outside click ─────────────────────────────────────────
function onOutsideClick(e: MouseEvent) {
  const el = document.querySelector('.node-picker') as HTMLElement | null
  if (el && !el.contains(e.target as Node)) emit('close')
}

onMounted(() => {
  nextTick(() => searchRef.value?.focus())
  document.addEventListener('mousedown', onOutsideClick)
})
onUnmounted(() => document.removeEventListener('mousedown', onOutsideClick))

// Reset active index when search changes
function onSearchInput() { activeIdx.value = 0 }
</script>

<template>
  <div class="node-picker" :style="style" @keydown="onKeyDown">
    <div class="picker-header">
      <svg class="picker-search-icon" viewBox="0 0 14 14" fill="none"
           stroke="currentColor" stroke-width="1.4">
        <circle cx="5.5" cy="5.5" r="4"/>
        <path d="M9 9l3.5 3.5"/>
      </svg>
      <input
        ref="searchRef"
        class="picker-search"
        v-model="search"
        placeholder="Search nodes…"
        @input="onSearchInput"
      />
    </div>

    <div class="picker-list">
      <template v-for="section in sections" :key="section.title">
        <div class="picker-category">{{ section.title }}</div>
        <button
          v-for="node in section.nodes"
          :key="node.nodeType"
          class="picker-item"
          :class="{ active: flatFiltered.indexOf(node) === activeIdx }"
          @mouseenter="activeIdx = flatFiltered.indexOf(node)"
          @click="emit('select', node.nodeType)"
        >
          <span class="picker-pip" :style="{ background: pip(section.title) }" />
          {{ node.label }}
        </button>
      </template>

      <div class="picker-empty" v-if="flatFiltered.length === 0">
        No nodes match "{{ search }}"
      </div>
    </div>
  </div>
</template>

<style scoped>
.node-picker {
  position: fixed;
  z-index: 500;
  width: 220px;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  box-shadow: var(--shadow-float);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* ── Header / search ────────────────────────────────────── */
.picker-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 10px;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-panel);
}

.picker-search-icon {
  width: 12px;
  height: 12px;
  color: var(--text-3);
  flex-shrink: 0;
}

.picker-search {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  font-size: 12px;
  color: var(--text-1);
  font-family: inherit;
}
.picker-search::placeholder { color: var(--text-3); }

/* ── List ───────────────────────────────────────────────── */
.picker-list {
  max-height: 280px;
  overflow-y: auto;
  padding: 4px 0;
}

.picker-category {
  padding: 6px 10px 3px;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-3);
}

.picker-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 5px 10px;
  background: transparent;
  border: none;
  color: var(--text-2);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  transition: background 0.08s, color 0.08s;
  border-radius: 0;
}
.picker-item:hover,
.picker-item.active {
  background: var(--bg-elevated);
  color: var(--text-1);
}

.picker-pip {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  opacity: 0.8;
}

.picker-empty {
  padding: 12px 10px;
  font-size: 11px;
  color: var(--text-3);
  text-align: center;
}
</style>
