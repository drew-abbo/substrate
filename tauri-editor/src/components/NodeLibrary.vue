<script setup lang="ts">
import { computed } from 'vue'

interface NodeEntry {
  nodeType: string
  label: string
  category: string
}

const props = defineProps<{
  nodes: NodeEntry[]
}>()

const emit = defineEmits<{
  addNode: [type: string]
}>()

const sections = computed(() => {
  const map = new Map<string, NodeEntry[]>()
  for (const node of props.nodes) {
    const cat = node.category || 'Other'
    const arr = map.get(cat) ?? []
    arr.push(node)
    map.set(cat, arr)
  }
  return [...map.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([title, nodes]) => ({
      title,
      nodes: [...nodes].sort((a, b) => a.label.localeCompare(b.label)),
    }))
})

const catColors: Record<string, string> = {
  input:       'var(--cat-source)',
  color:       'var(--cat-effect)',
  distortion:  'var(--cat-effect)',
  compositing: 'var(--cat-source)',
  glitch:      'var(--cat-analysis)',
  analysis:    'var(--cat-analysis)',
  transform:   'var(--cat-effect)',
  output:      'var(--cat-output)',
  audio:       'var(--cat-audio)',
}

function catColor(title: string): string {
  return catColors[title.toLowerCase()] ?? 'var(--cat-utility, #888)'
}
</script>

<template>
  <aside class="node-library">
    <div class="library-header">Nodes</div>

    <div v-if="nodes.length === 0" class="library-empty">Loading…</div>

    <div v-for="section in sections" :key="section.title" class="section">
      <div class="section-title">{{ section.title }}</div>
      <button
        v-for="node in section.nodes"
        :key="node.nodeType"
        class="node-entry"
        :style="{ '--entry-color': catColor(section.title) }"
        @click="emit('addNode', node.nodeType)"
      >
        <span class="entry-pip" />
        {{ node.label }}
      </button>
    </div>
  </aside>
</template>

<style scoped>
.node-library {
  display: flex;
  flex-direction: column;
  width: 188px;
  flex-shrink: 0;
  background: var(--bg-panel);
  border-right: 1px solid var(--border-subtle);
  overflow-y: auto;
  overflow-x: hidden;
}

.library-header {
  padding: 10px 12px 8px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-3);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.library-empty {
  padding: 12px;
  font-size: 11px;
  color: var(--text-3);
}

.section {
  padding: 6px 0;
  border-bottom: 1px solid var(--border-subtle);
}

.section-title {
  padding: 4px 12px 6px;
  font-size: 9.5px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
}

.node-entry {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 5px 12px;
  background: transparent;
  border: none;
  color: var(--text-2);
  font-size: 11.5px;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
  border-radius: 0;
}

.node-entry:hover {
  background: var(--bg-elevated);
  color: var(--text-1);
}

.entry-pip {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--entry-color);
  flex-shrink: 0;
  opacity: 0.85;
}

.node-entry:hover .entry-pip {
  opacity: 1;
}
</style>
