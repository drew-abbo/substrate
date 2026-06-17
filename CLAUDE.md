# Before making edits to vue or ts files

Make sure to check the styles folders so you are not overriding something that should match the global theme

# After making edits

cargo check -p (crate)

To fix format issues
cargo clippy -p (crate modified)

## After modifying vue or ts files

Make sure to fix lint errors
npm run lint

## File Structure

When creating modules alwasy organize the files like this.
src/
├── mod folder (example engine.rs)/
│   └── something the mod does
└── mod name.rs --other mods and uses
Example
src/
├── node/
│   └── node_definition.rs
└── node.rs

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **substrate** (3939 symbols, 9875 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> Index stale? Run `node .gitnexus/run.cjs analyze` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? `npx gitnexus analyze` (npm 11 crash → `npm i -g gitnexus`; #1939).

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows. For regression review, compare against the default branch: `detect_changes({scope: "compare", base_ref: "main"})`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/substrate/context` | Codebase overview, check index freshness |
| `gitnexus://repo/substrate/clusters` | All functional areas |
| `gitnexus://repo/substrate/processes` | All execution flows |
| `gitnexus://repo/substrate/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

---

# Development

## App Overview

This project ("Substrate") is two separate Tauri applications that work together:

- **tauri-launcher/** — Project manager UI. Lists, creates, renames, and deletes projects. Spawns the editor for a chosen project. Vite dev server on port **1421**.
- **tauri-editor/** — Node graph editor. Receives `--project <id>` as a CLI arg from the launcher. Vite dev server on port **1420**.

The launcher opens the editor binary with `--project <id>` so each editor window is scoped to one project.

> **Note:** `crates/launcher/` and `crates/launcher-core/` are **legacy egui code** — they are not used in normal development. The active launcher is `tauri-launcher/`.

## Running the Apps

All scripts are defined in the root `package.json` and use `concurrently` under the hood.

| Command | What it does |
|---------|-------------|
| `npm run dev` | Run both apps simultaneously (opens two Tauri windows) |
| `npm run dev:editor` | Run only the editor |
| `npm run dev:launcher` | Run only the launcher |
| `npm run lint` | Lint both apps |

## Architecture Overview

### tauri-editor/

- `src/` — Vue 3 frontend
  - `NodeGraph.vue` — Main canvas component (node graph rendering and interaction)
  - `styles/` — Global theme; check `theme.css` and `globals.css` before adding inline styles
- `src-tauri/src/` — Tauri/Rust backend
  - `engine_bridge.rs` — wgpu GPU integration (headless rendering, binary frame IPC)
  - `commands/` — Tauri commands grouped by domain: `graph`, `frame`, `nodes`, `output`, `project`
  - `state/` — App-wide state structs: `engine`, `frame`, `output`, `project`

### tauri-launcher/

- `src/` — Vue 3 frontend
  - `App.vue` — Project list view (main UI)
  - `styles/` — Global theme; check before adding inline styles
- `src-tauri/src/`
  - `commands.rs` — All 6 project management Tauri commands: `list`, `create`, `rename`, `delete`, `open`, `show_in_explorer`

### crates/ (shared Rust libraries)

| Crate | Purpose |
|-------|---------|
| `engine` | wgpu rendering core |
| `util` | `local_data` — project storage on disk |
| `media` | Video and audio handling |
| `editor-core` | Shared editor logic |
| `launcher-core` | Legacy — not used in active dev |

## Key Dev Notes

- After editing any `.vue` or `.ts` file, run `npm run lint` in the relevant app directory.
- After editing Rust in `tauri-editor/src-tauri/`, run `cargo check -p tauri-editor`.
- After editing Rust in `tauri-launcher/src-tauri/`, run `cargo check -p tauri-launcher`.
- Always check `src/styles/theme.css` and `src/styles/globals.css` before adding new styles — don't override the global theme with inline or component-scoped values.
