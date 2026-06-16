<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, provide } from 'vue'
import { VueFlow, useVueFlow } from '@vue-flow/core'
import type { Node, Edge } from '@vue-flow/core'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

import DynamicNode from './nodes/DynamicNode.vue'
import type { DynamicNodeData } from './nodes/DynamicNode.vue'
import OutputNode from './nodes/OutputNode.vue'
import NodePicker from './NodePicker.vue'

// ── Node definitions ──────────────────────────────────────
interface Widget {
  defaultFloat?: number
  defaultInt?:   number
  defaultBool?:  boolean
  min?:  number
  max?:  number
  step?: number
  choices?: string[]
  defaultChoiceIdx?: number
  useSlider: boolean
}

interface Port {
  id: string
  label: string
  portType: string
  showPin: boolean
  widget: Widget
}

interface NodeDef {
  nodeType:  string
  label:     string
  category:  string
  inputs:    Port[]
  outputs:   Port[]
}

interface RawWidget {
  default_float?: number; default_int?: number; default_bool?: boolean
  min?: number; max?: number; step?: number
  choices?: string[]; default_choice_idx?: number; use_slider: boolean
}

interface RawPort { id: string; label: string; port_type: string; show_pin: boolean; widget: RawWidget }
interface RawNodeDef { node_type: string; label: string; category: string; inputs: RawPort[]; outputs: RawPort[] }

function mapPort(p: RawPort): Port {
  return {
    id: p.id, label: p.label, portType: p.port_type, showPin: p.show_pin,
    widget: {
      defaultFloat: p.widget.default_float, defaultInt: p.widget.default_int,
      defaultBool: p.widget.default_bool, min: p.widget.min, max: p.widget.max,
      step: p.widget.step, choices: p.widget.choices,
      defaultChoiceIdx: p.widget.default_choice_idx, useSlider: p.widget.use_slider,
    },
  }
}

const OUTPUT_NODE_ID = 'output-node'

const nodeDefs = ref<NodeDef[]>([])
const nodeTypes = { dynamic: DynamicNode as any, output: OutputNode as any }

function makeNodeData(
  type: string,
  savedValues?: Record<string, number | boolean | string>,
): DynamicNodeData {
  const def = nodeDefs.value.find(d => d.nodeType === type)
  const defaults = defaultValues(def)
  return {
    nodeType: type,
    label:    def?.label    ?? type,
    category: def?.category ?? 'other',
    inputs:   def?.inputs   ?? [],
    outputs:  def?.outputs  ?? [],
    values:   savedValues ? { ...defaults, ...savedValues } : defaults,
  }
}

function defaultValues(def?: NodeDef): Record<string, number | boolean | string> {
  const values: Record<string, number | boolean | string> = {}
  for (const port of def?.inputs ?? []) {
    switch (port.portType) {
      case 'float': values[port.id] = port.widget.defaultFloat ?? 0; break
      case 'int':   values[port.id] = port.widget.defaultInt   ?? 0; break
      case 'bool':  values[port.id] = port.widget.defaultBool  ?? false; break
      case 'enum':  values[port.id] = port.widget.defaultChoiceIdx ?? 0; break
      case 'file':  values[port.id] = ''; break
    }
  }
  return values
}

// ── Vue-flow state ────────────────────────────────────────
const nodes = ref<Node[]>([{
  id: OUTPUT_NODE_ID,
  type: 'output',
  position: { x: 0, y: 0 },
  deletable: false,
  data: {},
}])
const edges = ref<Edge[]>([])

const {
  onConnect, addEdges, addNodes,
  removeNodes, removeEdges,
  getSelectedNodes, getSelectedEdges,
  onNodesChange, onEdgesChange, onNodeDragStop,
  project, fitView,
  getViewport, setViewport,
} = useVueFlow()

onConnect(p => {
  addEdges([{ ...p, type: 'smoothstep', style: { stroke: '#5a5a8a', strokeWidth: 1.5 } }])
  scheduleSync()
  markDirty()
})

// ── Engine sync + project save ────────────────────────────
let syncTimer: number | undefined
let unlistenClose: (() => void) | null = null

function doClose() {
  // Unlisten before asking Rust to close so Tauri goes through its normal
  // window-destruction path (giving WebView2 time to unregister its window
  // classes) instead of the abrupt app.exit(0) path we used before.
  unlistenClose?.()
  unlistenClose = null
  invoke('close_editor').catch(() => {})
}
// True once we've sent a non-empty graph so that a disconnect triggers a stop.
let engineIsRunning = false
// Suppresses engine sync during initial load (VueFlow emits 'add' events async on mount).
let isLoading = true

const isDirty           = ref(false)
const toastVisible      = ref(false)
const toastError        = ref('')
const closeDialogVisible = ref(false)
let toastTimer: number | undefined

function scheduleSync() {
  window.clearTimeout(syncTimer)
  syncTimer = window.setTimeout(syncGraph, 60)
}

function markDirty() {
  isDirty.value = true
}

// ── Saved-graph types (mirror commands/project.rs) ────────
interface SavedNode {
  id: string; nodeType: string; x: number; y: number
  values: Record<string, unknown>
}
interface SavedEdge {
  id: string; source: string; sourceHandle: string
  target: string; targetHandle: string
}
interface SavedViewport { x: number; y: number; zoom: number }
interface SavedGraph {
  nodes: SavedNode[]; edges: SavedEdge[]; viewport?: SavedViewport
}

type FlatNode = { id: string; position: { x: number; y: number }; data: unknown }
type FlatEdge = { id: string; source: string; sourceHandle?: string | null; target: string; targetHandle?: string | null }

async function saveGraph() {
  const vp = getViewport()
  const savedNodes: SavedNode[] = (nodes.value as FlatNode[]).map(n => ({
    id:       n.id,
    nodeType: n.id === OUTPUT_NODE_ID ? 'output' : (n.data as DynamicNodeData).nodeType,
    x:        n.position.x,
    y:        n.position.y,
    values:   n.id === OUTPUT_NODE_ID ? {} : (n.data as DynamicNodeData).values,
  }))
  const savedEdges: SavedEdge[] = (edges.value as FlatEdge[]).map(e => ({
    id:           e.id,
    source:       e.source,
    sourceHandle: e.sourceHandle ?? '',
    target:       e.target,
    targetHandle: e.targetHandle ?? '',
  }))
  const graph: SavedGraph = {
    nodes: savedNodes,
    edges: savedEdges,
    viewport: { x: vp.x, y: vp.y, zoom: vp.zoom },
  }
  try {
    await invoke('save_project', { graph })
    isDirty.value = false
    toastError.value = ''
  } catch (err) {
    const msg = String(err)
    console.warn('Save failed:', msg)
    toastError.value = `Save failed: ${msg}`
    toastVisible.value = true
    window.clearTimeout(toastTimer)
    toastTimer = window.setTimeout(() => { toastVisible.value = false; toastError.value = '' }, 4000)
  }
}

function showSavedToast() {
  toastError.value = ''
  toastVisible.value = true
  window.clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => { toastVisible.value = false }, 2000)
}

async function saveAndToast() {
  await saveGraph()
  showSavedToast()
}

// Resolved by dialog buttons; the onCloseRequested handler awaits this.
type CloseAction = 'save' | 'discard' | 'cancel'
let resolveClose: ((a: CloseAction) => void) | null = null

function dialogSave()    { resolveClose?.('save') }
function dialogDiscard() { resolveClose?.('discard') }
function dialogCancel()  { resolveClose?.('cancel') }

async function loadProject() {
  let saved: SavedGraph
  try {
    saved = await invoke<SavedGraph>('load_project')
  } catch (err) {
    console.warn('Load failed:', err)
    return
  }
  if (!saved.nodes.length) return

  // Cast through unknown to avoid VueFlow's deep Node/Edge generics triggering ts(2589)
  nodes.value = saved.nodes.map(sn => {
    if (sn.nodeType === 'output') {
      return { id: OUTPUT_NODE_ID, type: 'output', position: { x: sn.x, y: sn.y }, deletable: false, data: {} }
    }
    return {
      id:       sn.id,
      type:     'dynamic',
      position: { x: sn.x, y: sn.y },
      data:     makeNodeData(sn.nodeType, sn.values as Record<string, number | boolean | string>),
    }
  }) as unknown as Node[]
  edges.value = saved.edges.map(se => ({
    id:           se.id,
    source:       se.source,
    sourceHandle: se.sourceHandle,
    target:       se.target,
    targetHandle: se.targetHandle,
    type:         'smoothstep',
    style:        { stroke: '#5a5a8a', strokeWidth: 1.5 },
  })) as unknown as Edge[]
  if (saved.viewport) {
    await nextTick()
    setViewport({ x: saved.viewport.x, y: saved.viewport.y, zoom: saved.viewport.zoom })
  }
}

// Lets DynamicNode widgets trigger a sync when a value changes.
// Guard isLoading: widgets fire during initial render when loaded nodes mount.
provide('graphValueChanged', () => {
  if (isLoading) return
  scheduleSync()
  markDirty()
})

// Only used for engine sync — dirty tracking is done via explicit user-action calls below.
onNodesChange(changes => {
  if (isLoading) return
  if (changes.some(c => c.type === 'add' || c.type === 'remove')) scheduleSync()
})
onEdgesChange(changes => {
  if (isLoading) return
  if (changes.some(c => c.type === 'add' || c.type === 'remove')) scheduleSync()
})
onNodeDragStop(() => markDirty())

interface EdgePayload {
  fromNode: string
  fromOutput: string
  toNode: string
  toInput: string
}

async function syncGraph() {
  // The last edge into the output node wins, matching what the user sees.
  let outputSource: string | null = null
  const edgePayloads: EdgePayload[] = []
  for (const e of edges.value) {
    if (e.target === OUTPUT_NODE_ID) {
      outputSource = e.source
      continue
    }
    edgePayloads.push({
      fromNode:   e.source,
      fromOutput: e.sourceHandle ?? '',
      toNode:     e.target,
      toInput:    e.targetHandle ?? '',
    })
  }

  // Only drive the engine when something is wired to the output node.
  // If nothing is connected and the engine wasn't running, skip entirely.
  // If nothing is connected but the engine WAS running (user just disconnected),
  // send an empty graph so the engine stops.
  if (!outputSource && !engineIsRunning) return

  const nodePayloads: { id: string; nodeType: string; values: DynamicNodeData['values'] }[] = []
  if (outputSource) {
    for (const n of nodes.value) {
      if (n.id === OUTPUT_NODE_ID) continue
      const data = n.data as DynamicNodeData
      nodePayloads.push({ id: n.id, nodeType: data.nodeType, values: data.values })
    }
  }

  engineIsRunning = outputSource !== null

  const payload = {
    nodes: nodePayloads,
    edges: outputSource ? edgePayloads : [],
    outputSource,
  }
  try {
    await invoke('update_graph', { payload })
  } catch (err) {
    console.warn('Graph sync failed:', err)
  }
}

// ── Popup state ───────────────────────────────────────────
interface ScreenFlowPos { x: number; y: number; flowX: number; flowY: number }

const canvasMenu = ref<ScreenFlowPos | null>(null)
const picker     = ref<ScreenFlowPos | null>(null)
const nodeMenu   = ref<{ nodeId: string; x: number; y: number } | null>(null)

const anyOpen = () => !!(canvasMenu.value || picker.value || nodeMenu.value)

function closeAll() {
  canvasMenu.value = null
  picker.value     = null
  nodeMenu.value   = null
}

// ── Canvas right-click → context menu ────────────────────
function onPaneContextMenu(e: MouseEvent) {
  e.preventDefault()
  closeAll()
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  const flow = project({ x: e.clientX - rect.left, y: e.clientY - rect.top })
  canvasMenu.value = { x: e.clientX, y: e.clientY, flowX: flow.x, flowY: flow.y }
}

function openPicker() {
  if (!canvasMenu.value) return
  picker.value     = { ...canvasMenu.value }
  canvasMenu.value = null
}

function doFitView() {
  fitView({ padding: 0.15, duration: 300 })
  closeAll()
}

function onFlowInit() {
  nextTick(() => fitView({ padding: 0.35, duration: 0 }))
}

// ── Node picker confirm ───────────────────────────────────
function onPickerSelect(type: string) {
  if (!picker.value) return
  addNodes([{
    id:       `node-${Date.now()}`,
    type:     'dynamic',
    position: { x: picker.value.flowX, y: picker.value.flowY },
    data:     makeNodeData(type),
  }])
  markDirty()
  picker.value = null
}

// ── Node right-click ──────────────────────────────────────
function onNodeContextMenu({ event, node }: { event: MouseEvent | TouchEvent; node: Node }) {
  event.preventDefault()
  event.stopPropagation()
  closeAll()
  const me = event as MouseEvent
  nodeMenu.value = { nodeId: node.id, x: me.clientX ?? 0, y: me.clientY ?? 0 }
}

function deleteNode(id: string) {
  if (id === OUTPUT_NODE_ID) { nodeMenu.value = null; return }
  removeNodes([{ id } as any])
  markDirty()
  nodeMenu.value = null
}

// ── Edge right-click → instant disconnect ────────────────
function onEdgeContextMenu({ event, edge }: { event: MouseEvent | TouchEvent; edge: Edge }) {
  event.preventDefault()
  event.stopPropagation()
  removeEdges([{ id: edge.id } as any])
  markDirty()
}

// ── Keyboard ──────────────────────────────────────────────
function onKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    saveAndToast()
    return
  }
  const tag = (e.target as HTMLElement).tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA') return
  if (e.key === 'Delete' || e.key === 'Backspace') {
    removeNodes(getSelectedNodes.value.filter(n => n.id !== OUTPUT_NODE_ID))
    removeEdges(getSelectedEdges.value)
    markDirty()
  }
  if (e.key === 'Escape') closeAll()
}

// ── Menu position (flip near viewport edge) ───────────────
function menuStyle(x: number, y: number) {
  const vw = document.documentElement.clientWidth
  const vh = document.documentElement.clientHeight
  return {
    left: (x + 160 > vw ? x - 160 : x) + 'px',
    top:  (y + 120 > vh ? y - 120 : y) + 'px',
  }
}

// ── Exposed ───────────────────────────────────────────────
function addNode(type: string) {
  addNodes([{
    id:       `node-${Date.now()}`,
    type:     'dynamic',
    position: { x: 200 + Math.random() * 100, y: 100 + Math.random() * 80 },
    data:     makeNodeData(type),
  }])
  markDirty()
}
defineExpose({ addNode })

// ── Lifecycle ─────────────────────────────────────────────
onMounted(async () => {
  window.addEventListener('keydown', onKeyDown)
  unlistenClose = await getCurrentWindow().onCloseRequested(async event => {
    // Always prevent Tauri's default — in Tauri 2 NOT calling preventDefault()
    // causes Tauri to internally re-call close(), which re-fires this handler
    // and infinite-loops. We handle closing explicitly in every path instead.
    event.preventDefault()

    if (!isDirty.value) {
      doClose()
      return
    }

    // Pause on unsaved changes: show dialog and wait for user's decision.
    const action = await new Promise<CloseAction>(resolve => {
      resolveClose = resolve
      closeDialogVisible.value = true
    })
    closeDialogVisible.value = false
    resolveClose = null

    if (action === 'cancel') return

    if (action === 'save') {
      await saveGraph()
    }

    isDirty.value = false
    doClose()
  })
  try {
    const raw = await invoke<RawNodeDef[]>('get_node_definitions')
    nodeDefs.value = raw.map(d => ({
      nodeType:  d.node_type,
      label:     d.label,
      category:  d.category,
      inputs:    d.inputs.map(mapPort),
      outputs:   d.outputs.map(mapPort),
    }))
    await loadProject()
    // Wait for VueFlow's async 'add' change events to fire and be ignored by isLoading,
    // then clear the flag so subsequent user changes are tracked normally.
    await nextTick()
    await nextTick()
    isLoading = false
    isDirty.value = false
    syncGraph()
  } catch (err) {
    console.warn('Could not load node definitions:', err)
    isLoading = false
  }
})
onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
  unlistenClose?.()
})
</script>

<template>
  <div class="graph-wrap" tabindex="0">

    <VueFlow
      v-model:nodes="nodes"
      v-model:edges="edges"
      :node-types="nodeTypes"
      :default-zoom="1"
      :min-zoom="0.15"
      :max-zoom="4"
      class="vf"
      @init="onFlowInit"
      @pane-context-menu="onPaneContextMenu"
      @node-context-menu="onNodeContextMenu"
      @edge-context-menu="onEdgeContextMenu"
    />

    <!-- Backdrop: closes all popups when clicking outside them -->
    <div v-if="anyOpen()" class="popup-backdrop" @click.stop="closeAll" @contextmenu.prevent="closeAll" />

    <!-- Canvas right-click menu -->
    <div v-if="canvasMenu" class="ctx-menu" :style="menuStyle(canvasMenu.x, canvasMenu.y)">
      <button class="ctx-item" @click.stop="openPicker">
        <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
          <circle cx="7" cy="7" r="5.5"/>
          <path d="M7 4.5v5M4.5 7h5"/>
        </svg>
        Add Node
      </button>
      <div class="ctx-divider" />
      <button class="ctx-item" @click.stop="doFitView">
        <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
          <path d="M1 4V1h3M10 1h3v3M13 10v3h-3M4 13H1v-3"/>
          <rect x="4" y="4" width="6" height="6" rx="0.5"/>
        </svg>
        Fit View
      </button>
    </div>

    <!-- Node right-click menu -->
    <div v-if="nodeMenu" class="ctx-menu" :style="menuStyle(nodeMenu.x, nodeMenu.y)">
      <template v-if="nodeMenu.nodeId === OUTPUT_NODE_ID">
        <div class="ctx-item ctx-locked">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
            <rect x="3" y="6" width="8" height="6" rx="1"/>
            <path d="M5 6V4a3 3 0 0 1 4 0v2"/>
          </svg>
          Locked
        </div>
      </template>
      <template v-else>
        <button class="ctx-item danger" @click.stop="deleteNode(nodeMenu.nodeId)">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
            <path d="M2 4h10M5 4V2h4v2M6 7v4M8 7v4M3 4l1 8h6l1-8"/>
          </svg>
          Delete Node
        </button>
      </template>
    </div>

    <!-- Node picker -->
    <NodePicker
      v-if="picker"
      :x="picker.x"
      :y="picker.y"
      :nodes="nodeDefs"
      @select="onPickerSelect"
      @close="picker = null"
    />

    <!-- Saved / error toast -->
    <Transition name="toast">
      <div v-if="toastVisible" class="save-toast" :class="{ error: toastError }">
        {{ toastError || 'Saved' }}
      </div>
    </Transition>

    <!-- Unsaved-changes dialog -->
    <div v-if="closeDialogVisible" class="dialog-backdrop">
      <div class="dialog-box">
        <p class="dialog-msg">You have unsaved changes. Save before closing?</p>
        <div class="dialog-actions">
          <button class="dialog-btn primary" @click="dialogSave">Save</button>
          <button class="dialog-btn"         @click="dialogDiscard">Don't Save</button>
          <button class="dialog-btn"         @click="dialogCancel">Cancel</button>
        </div>
      </div>
    </div>

  </div>
</template>

<style>
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
</style>

<style scoped>
.graph-wrap {
  width: 100%;
  height: 100%;
  background: var(--bg-canvas);
  position: relative;
  outline: none;
}
.vf { width: 100%; height: 100%; }

/* ── Backdrop ───────────────────────────────────────────── */
.popup-backdrop {
  position: fixed;
  inset: 0;
  z-index: 399;
}

/* ── Shared context menu ────────────────────────────────── */
.ctx-menu {
  position: fixed;
  z-index: 400;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 5px;
  box-shadow: var(--shadow-popup);
  padding: 4px 0;
  min-width: 160px;
}

.ctx-divider {
  height: 1px;
  background: var(--border-subtle);
  margin: 3px 0;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 12px;
  background: transparent;
  border: none;
  color: var(--text-2);
  font-size: 12px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.08s, color 0.08s;
}
.ctx-item:hover         { background: var(--bg-elevated); color: var(--text-1); }
.ctx-item.danger:hover  { color: var(--cat-output); }
.ctx-item svg           { width: 13px; height: 13px; flex-shrink: 0; }
.ctx-locked             { color: var(--text-3); cursor: default; pointer-events: none; }

/* ── Saved toast ────────────────────────────────────────── */
.save-toast {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  padding: 7px 18px;
  font-size: 12px;
  color: var(--text-2);
  box-shadow: var(--shadow-popup);
  pointer-events: none;
  z-index: 500;
}
.save-toast.error { color: var(--cat-output, #e05); border-color: var(--cat-output, #e05); }
.toast-enter-active, .toast-leave-active { transition: opacity 0.2s, transform 0.2s; }
.toast-enter-from { opacity: 0; transform: translateX(-50%) translateY(6px); }
.toast-leave-to   { opacity: 0; transform: translateX(-50%) translateY(6px); }

/* ── Unsaved-changes dialog ─────────────────────────────── */
.dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 600;
}
.dialog-box {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 24px 28px 20px;
  box-shadow: var(--shadow-popup);
  min-width: 300px;
  max-width: 380px;
}
.dialog-msg {
  margin: 0 0 20px;
  font-size: 13px;
  color: var(--text-1);
  line-height: 1.5;
}
.dialog-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.dialog-btn {
  padding: 6px 16px;
  border-radius: 5px;
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text-2);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.08s, color 0.08s;
}
.dialog-btn:hover { background: var(--bg-canvas); color: var(--text-1); }
.dialog-btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.dialog-btn.primary:hover { filter: brightness(1.15); }
</style>

<style>
.vue-flow__pane        { background: transparent; }
.vue-flow__background  { display: none; }
.vue-flow__edge-path   { stroke-opacity: 0.75; filter: drop-shadow(0 0 3px rgba(0,0,0,0.6)); }
.vue-flow__edge.selected .vue-flow__edge-path { stroke-opacity: 1; }
.vue-flow__connection-path { stroke: var(--connection-stroke); stroke-width: 1.5; stroke-dasharray: 6 4; opacity: 0.6; }
.vue-flow__selection { background: var(--accent-dim); border: 1px solid var(--accent); }
</style>
