<script setup lang="ts">
import { inject } from 'vue'
import { Handle, Position, useVueFlow } from '@vue-flow/core'
import { open as openFilePicker } from '@tauri-apps/plugin-dialog'
import { outputHandleTop, inputHandleTop } from '../../composables/nodeLayout'

export interface Widget {
  defaultFloat?: number
  defaultInt?: number
  defaultBool?: boolean
  min?: number
  max?: number
  step?: number
  choices?: string[]
  defaultChoiceIdx?: number
  useSlider: boolean
}

export interface Port {
  id: string
  label: string
  portType: string
  showPin: boolean
  widget: Widget
}

export interface DynamicNodeData {
  nodeType: string
  label: string
  category: string
  inputs: Port[]
  outputs: Port[]
  /** Current widget values keyed by input port id. */
  values: Record<string, number | boolean | string>
}

const props = defineProps<{
  id: string
  data: DynamicNodeData
  selected?: boolean
}>()

const { updateNodeData } = useVueFlow()

/** Provided by NodeGraph; pushes the graph to the engine (debounced). */
const onValueChange = inject<() => void>('graphValueChanged', () => {})

function setValue(portId: string, value: number | boolean | string) {
  updateNodeData(props.id, { values: { ...props.data.values, [portId]: value } })
  onValueChange()
}


function numberFrom(e: Event): number {
  return Number((e.target as HTMLInputElement).value)
}

async function browseFile(portId: string) {
  const path = await openFilePicker({
    multiple: false,
    filters: [
      { name: 'Media', extensions: ['mp4', 'mov', 'avi', 'webm', 'mkv', 'png', 'jpg', 'jpeg', 'gif', 'bmp', 'tiff', 'webp'] },
    ],
  })
  if (typeof path === 'string') setValue(portId, path)
}

function fileLabel(val: unknown): string {
  const p = String(val ?? '')
  if (!p) return 'Browse…'
  const parts = p.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] ?? 'Browse…'
}

const catIconPaths: Record<string, string> = {
  input:       'M1 4h9l5 4-5 4H1V4z',
  sources:     'M1 4h9l5 4-5 4H1V4z',
  audio:       'M6 1v10M10 3v6M2 4v4M14 4v4',
  color:       'M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zM5 8a3 3 0 0 0 6 0',
  distortion:  'M1 8c2-4 4 4 6 0s4-4 6 0',
  glitch:      'M1 5h4l2-3 2 6 2-3h2M1 9h3l2 4 2-8 2 4h3',
  compositing: 'M3 3h8v8H3zM7 7h6v6H7z',
  transform:   'M2 2l10 0M7 2v10M2 12l10 0',
  analysis:    'M1 8h2l2-4 2 8 2-6 2 4 2-2h2',
  output:      'M1 2h14v9H1zM5 11v3M11 11v3M3 14h10',
  effects:     'M8 2a6 6 0 1 0 0 12A6 6 0 0 0 8 2zM8 5v3l3 1.5',
}

function iconPath(cat: string): string {
  return catIconPaths[cat.toLowerCase()] ?? catIconPaths.effects
}
</script>

<template>
  <div class="node-card" :class="{ selected }" :data-cat="data.category.toLowerCase()">

    <!-- Output handles (right side) -->
    <Handle
      v-for="(port, i) in data.outputs"
      :key="`out-${port.id}`"
      :id="port.id"
      type="source"
      :position="Position.Right"
      :class="['port-handle', 'right', port.portType]"
      :style="{ top: outputHandleTop(i) }"
    />

    <!-- Input handles (left side, only for connectable ports) -->
    <template v-for="(port, i) in data.inputs" :key="`in-h-${port.id}`">
      <Handle
        v-if="port.showPin"
        :id="port.id"
        type="target"
        :position="Position.Left"
        :class="['port-handle', port.portType]"
        :style="{ top: inputHandleTop(i, data.outputs.length) }"
      />
    </template>

    <!-- Header -->
    <div class="node-header">
      <svg class="node-icon" viewBox="0 0 16 16" fill="none"
           stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
        <path :d="iconPath(data.category)" />
      </svg>
      <span class="node-title">{{ data.label }}</span>
    </div>

    <!-- Output rows -->
    <div
      v-for="port in data.outputs"
      :key="`out-row-${port.id}`"
      class="port-row port-row-out"
    >
      <span class="port-label">{{ port.label }}</span>
    </div>

    <!-- Input rows -->
    <div
      v-for="port in data.inputs"
      :key="`in-row-${port.id}`"
      class="port-row port-row-in"
    >
      <span class="port-label">{{ port.label }}</span>

      <!-- Float: slider or number -->
      <template v-if="port.portType === 'float'">
        <input
          v-if="port.widget.useSlider && port.widget.min != null && port.widget.max != null"
          type="range"
          class="node-field-slider"
          :min="port.widget.min"
          :max="port.widget.max"
          :step="port.widget.step ?? 0.01"
          :value="Number(data.values[port.id] ?? port.widget.defaultFloat ?? 0)"
          @input="setValue(port.id, numberFrom($event))"
          @mousedown.stop
        />
        <input
          v-else
          type="number"
          class="node-field-num"
          :min="port.widget.min"
          :max="port.widget.max"
          :step="port.widget.step ?? 0.1"
          :value="Number(data.values[port.id] ?? port.widget.defaultFloat ?? 0)"
          @input="setValue(port.id, numberFrom($event))"
          @mousedown.stop
        />
      </template>

      <!-- Int -->
      <input
        v-else-if="port.portType === 'int'"
        type="number"
        class="node-field-num"
        :min="port.widget.min"
        :max="port.widget.max"
        :step="port.widget.step ?? 1"
        :value="Number(data.values[port.id] ?? port.widget.defaultInt ?? 0)"
        @input="setValue(port.id, Math.round(numberFrom($event)))"
        @mousedown.stop
      />

      <!-- Bool -->
      <input
        v-else-if="port.portType === 'bool'"
        type="checkbox"
        class="node-field-check"
        :checked="Boolean(data.values[port.id] ?? port.widget.defaultBool ?? false)"
        @change="setValue(port.id, ($event.target as HTMLInputElement).checked)"
        @mousedown.stop
      />

      <!-- Enum -->
      <select
        v-else-if="port.portType === 'enum'"
        class="node-field-select"
        @change="setValue(port.id, numberFrom($event))"
        @mousedown.stop
      >
        <option
          v-for="(choice, ci) in (port.widget.choices ?? [])"
          :key="ci"
          :value="ci"
          :selected="ci === Number(data.values[port.id] ?? port.widget.defaultChoiceIdx ?? 0)"
        >{{ choice }}</option>
      </select>

      <!-- File -->
      <button
        v-else-if="port.portType === 'file'"
        class="node-field-file"
        @click.stop="browseFile(port.id)"
        @mousedown.stop
      >{{ fileLabel(data.values[port.id]) }}</button>

    </div>
  </div>
</template>

<style scoped>
/* ── Output rows ─────────────────────────────────────────── */
.port-row-out {
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 18px 0 10px;
}

/* ── Input rows ──────────────────────────────────────────── */
.port-row-in {
  height: 26px;
  display: flex;
  align-items: center;
  padding: 0 8px 0 14px;
  gap: 6px;
}

/* Label never grows so the widget takes available space */
.port-row-in .port-label {
  flex-shrink: 0;
}

/* ── Field widgets ───────────────────────────────────────── */
.node-field-num {
  width: 64px;
  flex-shrink: 0;
  height: 18px;
  background: var(--bg-app);
  border: 1px solid var(--border-default);
  border-radius: 3px;
  color: var(--text-1);
  font-size: 10px;
  font-family: inherit;
  padding: 0 4px;
  text-align: right;
}
.node-field-num:focus { outline: 1px solid var(--accent); border-color: var(--accent); }

.node-field-slider {
  flex: 1;
  min-width: 0;
  accent-color: var(--accent);
  cursor: pointer;
}

.node-field-check {
  width: 13px;
  height: 13px;
  accent-color: var(--accent);
  cursor: pointer;
  flex-shrink: 0;
}

.node-field-select {
  flex: 1;
  min-width: 0;
  height: 18px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 3px;
  color: var(--text-1);
  font-size: 10px;
  font-family: inherit;
  padding: 0 2px;
}
.node-field-select:focus { outline: 1px solid var(--accent); }

.node-field-file {
  flex: 1;
  min-width: 0;
  height: 18px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 3px;
  color: var(--text-2);
  font-size: 9px;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding: 0 6px;
}
.node-field-file:hover { color: var(--text-1); border-color: var(--border-strong); }
</style>
