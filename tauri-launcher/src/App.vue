<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import TitleBar from './components/TitleBar.vue'

interface Project {
  id: string
  name: string
  created: string
  lastEdited: string | null
}

interface ProjectRow extends Project {
  editName: string
}

const projects = ref<ProjectRow[]>([])
const search    = ref('')
const loading   = ref(false)
const errorMsg  = ref<string | null>(null)

const creating  = ref(false)
const newName   = ref('')
const newInput  = ref<HTMLInputElement | null>(null)

const deletingProject = ref<ProjectRow | null>(null)
const keepOpen = ref(false)

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return projects.value
  return projects.value.filter(p => p.name.toLowerCase().includes(q))
})

function showError(e: unknown) { errorMsg.value = String(e) }

function toRow(p: Project): ProjectRow {
  return { ...p, editName: p.name }
}

async function loadProjects() {
  loading.value = true
  try {
    const raw = await invoke<Project[]>('list_projects')
    projects.value = raw.map(toRow)
  } catch (e) { showError(e) }
  finally { loading.value = false }
}

async function openProject(id: string) {
  try { await invoke('open_project', { id, keepOpen: keepOpen.value }) }
  catch (e) { showError(e) }
}

function startCreate() {
  creating.value = true
  newName.value  = ''
  nextTick(() => newInput.value?.focus())
}

function cancelCreate() {
  creating.value = false
  newName.value  = ''
}

async function confirmCreate() {
  const name = newName.value.trim()
  if (!name) { creating.value = false; return }
  try {
    const p = await invoke<Project>('create_project', { name })
    projects.value.unshift(toRow(p))
    creating.value = false
  } catch (e) { showError(e) }
}

async function commitName(row: ProjectRow) {
  const name = row.editName.trim()
  if (!name || name === row.name) {
    row.editName = row.name
    return
  }
  try {
    await invoke('rename_project', { id: row.id, name })
    row.name = name
  } catch (e) {
    row.editName = row.name
    showError(e)
  }
}

function revertAndBlur(row: ProjectRow, e: KeyboardEvent) {
  row.editName = row.name
  ;(e.target as HTMLInputElement).blur()
}

function blurTarget(e: KeyboardEvent) {
  ;(e.target as HTMLInputElement).blur()
}

async function showInExplorer(id: string) {
  try { await invoke('show_project_in_explorer', { id }) }
  catch (e) { showError(e) }
}

async function doDelete() {
  if (!deletingProject.value) return
  const { id } = deletingProject.value
  try {
    await invoke('delete_project', { id })
    projects.value = projects.value.filter(p => p.id !== id)
  } catch (e) { showError(e) }
  deletingProject.value = null
}

onMounted(loadProjects)
</script>

<template>
  <div class="launcher-app">
    <TitleBar />

    <div class="content">
      <!-- Search bar -->
      <div class="search-bar">
        <svg class="search-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="6.5" cy="6.5" r="4.5"/>
          <path d="m10.5 10.5 3 3"/>
        </svg>
        <input
          v-model="search"
          type="search"
          placeholder="Filter projects…"
          class="search-input"
        />
      </div>

      <!-- Project list -->
      <div class="project-list">

        <!-- Creating row -->
        <div v-if="creating" class="project-row creating-row">
          <span class="row-open-btn row-icon-placeholder">
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.2">
              <rect x="4" y="3" width="12" height="14" rx="2"/>
              <path d="M7 8h6M7 11h4"/>
            </svg>
          </span>
          <div class="row-content">
            <input
              ref="newInput"
              v-model="newName"
              class="project-name-input is-creating"
              placeholder="Project name…"
              @blur="confirmCreate"
              @keydown.enter.prevent="blurTarget($event)"
              @keydown.esc.prevent="cancelCreate"
            />
          </div>
        </div>

        <!-- Existing rows -->
        <div
          v-for="row in filtered"
          :key="row.id"
          class="project-row"
        >
          <button
            class="row-open-btn"
            title="Open project"
            @click="openProject(row.id)"
          >
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.2">
              <rect x="4" y="3" width="12" height="14" rx="2"/>
              <path d="M7 8h6M7 11h4"/>
            </svg>
          </button>

          <div class="row-content">
            <input
              v-model="row.editName"
              class="project-name-input"
              :title="row.name"
              @blur="commitName(row)"
              @keydown.enter.prevent="blurTarget($event)"
              @keydown.esc="revertAndBlur(row, $event)"
            />
            <span class="row-meta">{{ row.lastEdited ?? row.created }}</span>
          </div>

          <div class="row-actions">
            <button
              class="icon-btn"
              title="Show in file explorer"
              @click="showInExplorer(row.id)"
            >
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
                <path d="M2 5.5h12v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-7z"/>
                <path d="M2 5.5 3 3.5h4l1 2"/>
              </svg>
            </button>

            <button
              class="icon-btn danger"
              title="Delete project"
              @click="deletingProject = row"
            >
              <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
                <path d="M2 4h10M5 4V2h4v2M6 7v4M8 7v4M3 4l1 8h6l1-8"/>
              </svg>
            </button>
          </div>
        </div>

        <!-- Empty state -->
        <div v-if="filtered.length === 0 && !creating && !loading" class="empty-state">
          <svg viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1">
            <rect x="8" y="6" width="32" height="36" rx="3"/>
            <path d="M24 18v12M18 24h12"/>
          </svg>
          <p v-if="search">No projects match "{{ search }}"</p>
          <p v-else>No projects yet — click <strong>New Project</strong> to get started</p>
        </div>

        <div v-if="loading" class="loading">Loading…</div>
      </div>

      <!-- Bottom bar -->
      <div class="bottom-bar">
        <button class="btn-new" @click="startCreate">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
            <circle cx="7" cy="7" r="5.5"/>
            <path d="M7 4.5v5M4.5 7h5"/>
          </svg>
          New Project
        </button>

        <label class="keep-open-label">
          <input v-model="keepOpen" type="checkbox" class="keep-open-check" />
          Keep launcher open
        </label>
      </div>
    </div>

    <!-- Error toast -->
    <div v-if="errorMsg" class="error-toast" @click="errorMsg = null">
      {{ errorMsg }}
    </div>

    <!-- Delete confirm modal -->
    <Teleport to="body">
      <div v-if="deletingProject" class="modal-backdrop" @click="deletingProject = null">
        <div class="modal" @click.stop>
          <h3 class="modal-title">Delete Project</h3>
          <p class="modal-body">
            Delete <strong>{{ deletingProject.name }}</strong>?
            This cannot be undone.
          </p>
          <div class="modal-actions">
            <button class="btn-sm" @click="deletingProject = null">Cancel</button>
            <button class="btn-sm danger" @click="doDelete">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style>
@import './styles/theme.css';
@import './styles/globals.css';
</style>

<style scoped>
.launcher-app {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
}

/* ── Content ─────────────────────────────────────────────── */
.content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-canvas);
}

/* ── Search bar ──────────────────────────────────────────── */
.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.search-icon {
  width: 13px;
  height: 13px;
  color: var(--text-3);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  height: 30px;
  padding: 0 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 4px;
  color: var(--text-1);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.1s;
}
.search-input:focus { border-color: var(--accent); }
.search-input::placeholder { color: var(--text-3); }
.search-input::-webkit-search-cancel-button { display: none; }

/* ── Project list ────────────────────────────────────────── */
.project-list {
  flex: 1;
  overflow-y: auto;
}

/* ── Row ─────────────────────────────────────────────────── */
.project-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 0 14px;
  border-bottom: 1px solid var(--border-subtle);
  position: relative;
  transition: background 0.1s;
}
.project-row:hover { background: rgba(255, 255, 255, 0.05); }
.project-row:focus-within { background: rgba(0, 204, 168, 0.04); }

.creating-row {
  background: rgba(0, 204, 168, 0.04) !important;
  border-bottom-color: rgba(0, 204, 168, 0.15);
}

/* Open project button (left icon) */
.row-open-btn {
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
  color: var(--text-3);
  cursor: pointer;
  padding: 0;
  transition: background 0.1s, color 0.1s, border-color 0.1s;
}
.row-open-btn:hover {
  background: var(--accent-dim);
  border-color: rgba(0, 204, 168, 0.3);
  color: var(--accent);
}
.row-open-btn svg { width: 17px; height: 17px; }

.row-icon-placeholder {
  opacity: 0.35;
  cursor: default;
  pointer-events: none;
}

/* ── Row content (name + date stacked) ───────────────────── */
.row-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 2px;
}

/* ── Name input ──────────────────────────────────────────── */
.project-name-input {
  width: 100%;
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  color: var(--text-1);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 2px 5px;
  outline: none;
  cursor: text;
  transition: background 0.12s, border-color 0.12s;
  line-height: 1.3;
}
.project-name-input:hover:not(:focus) {
  border-color: var(--border-default);
  background: rgba(255, 255, 255, 0.04);
}
.project-name-input:focus {
  background: var(--bg-elevated);
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(0, 204, 168, 0.1);
}
.project-name-input::placeholder { color: var(--text-3); font-weight: 400; }
.project-name-input::selection { background: rgba(0, 204, 168, 0.25); }
.project-name-input.is-creating {
  border-color: rgba(0, 204, 168, 0.4);
  background: var(--bg-elevated);
}
.project-name-input.is-creating:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(0, 204, 168, 0.1);
}

/* ── Meta date ───────────────────────────────────────────── */
.row-meta {
  font-size: 12px;
  color: var(--text-2);
  padding-left: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── Action buttons group ────────────────────────────────── */
.row-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

/* ── Icon buttons ────────────────────────────────────────── */
.icon-btn {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 5px;
  color: var(--text-2);
  cursor: pointer;
  padding: 0;
  transition: background 0.1s, color 0.1s, border-color 0.1s;
}
.icon-btn:hover {
  background: var(--bg-elevated);
  color: var(--text-1);
  border-color: var(--border-default);
}
.icon-btn.danger:hover {
  color: #f43f5e;
  border-color: rgba(244, 63, 94, 0.35);
  background: rgba(244, 63, 94, 0.08);
}
.icon-btn svg { width: 14px; height: 14px; }

/* ── Empty / Loading ─────────────────────────────────────── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 20px;
  color: var(--text-3);
  text-align: center;
}
.empty-state svg { width: 40px; height: 40px; opacity: 0.25; }
.empty-state p { font-size: 12px; line-height: 1.6; }
.empty-state strong { color: var(--text-2); }

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
  color: var(--text-3);
  font-size: 12px;
}

/* ── Bottom bar ──────────────────────────────────────────── */
.bottom-bar {
  flex-shrink: 0;
  padding: 10px 14px;
  border-top: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.btn-new {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 14px;
  background: var(--accent-dim);
  border: 1px solid rgba(0, 204, 168, 0.25);
  border-radius: 5px;
  color: var(--accent);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s, border-color 0.1s;
}
.btn-new:hover {
  background: rgba(0, 204, 168, 0.14);
  border-color: rgba(0, 204, 168, 0.45);
}
.btn-new svg { width: 12px; height: 12px; flex-shrink: 0; }

/* ── Keep-open checkbox ──────────────────────────────────── */
.keep-open-label {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  color: var(--text-2);
  cursor: pointer;
  user-select: none;
  transition: color 0.1s;
}
.keep-open-label:hover { color: var(--text-1); }

.keep-open-check {
  appearance: none;
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  background: var(--bg-elevated);
  cursor: pointer;
  flex-shrink: 0;
  position: relative;
  transition: background 0.1s, border-color 0.1s;
}
.keep-open-check:hover { border-color: var(--accent); }
.keep-open-check:checked {
  background: var(--accent);
  border-color: var(--accent);
}
.keep-open-check:checked::after {
  content: '';
  position: absolute;
  left: 3px;
  top: 1px;
  width: 6px;
  height: 4px;
  border-left: 1.5px solid #07070f;
  border-bottom: 1.5px solid #07070f;
  transform: rotate(-45deg);
}

/* ── Error toast ─────────────────────────────────────────── */
.error-toast {
  position: fixed;
  bottom: 14px;
  left: 50%;
  transform: translateX(-50%);
  background: #2a1010;
  border: 1px solid rgba(244, 63, 94, 0.4);
  border-radius: 6px;
  color: #f43f5e;
  font-size: 11px;
  padding: 8px 16px;
  cursor: pointer;
  z-index: 500;
  max-width: 480px;
  text-align: center;
}

/* ── Delete modal ────────────────────────────────────────── */
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 600;
}

.modal {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: var(--shadow-float);
  padding: 24px;
  width: 320px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.modal-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
}

.modal-body {
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.6;
}
.modal-body strong { color: var(--text-1); }

.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 4px;
}

.btn-sm {
  height: 26px;
  padding: 0 14px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 4px;
  color: var(--text-2);
  font-size: 11px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}
.btn-sm:hover { background: var(--bg-surface); color: var(--text-1); }
.btn-sm.danger { border-color: rgba(244, 63, 94, 0.3); color: #f43f5e; }
.btn-sm.danger:hover { background: rgba(244, 63, 94, 0.1); }
</style>
