use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use engine::engine_outpost::EngineCommand;
use engine::node::NodeInputKind;
use engine::node_graph::{InputValue, NodeGraph};
use serde::Deserialize;
use tauri::State;

use crate::state::engine_state::EngineState;
use crate::state::frame_state::FrameState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPayload {
    nodes: Vec<NodePayload>,
    edges: Vec<EdgePayload>,
    /// Frontend id of the node feeding the output monitor, if any.
    output_source: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePayload {
    id: String,
    node_type: String,
    #[serde(default)]
    values: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgePayload {
    from_node: String,
    from_output: String,
    to_node: String,
    to_input: String,
}

/// Push the current frontend graph to the engine and update the output node.
///
/// Mirrors what the egui editor's `push_graph_to_engine` did: rebuild the
/// engine [`NodeGraph`], send `UpdateGraph` followed by `SetOutputNode`.
#[tauri::command]
pub fn update_graph(
    payload: GraphPayload,
    engine: State<'_, EngineState>,
    frames: State<'_, FrameState>,
) -> Result<(), String> {
    let library = engine
        .library
        .lock()
        .unwrap()
        .clone()
        .ok_or("engine not ready")?;

    let mut node_ids = engine.node_ids.lock().unwrap();

    // Drop mappings for nodes deleted on the frontend.
    let live: HashSet<&String> = payload.nodes.iter().map(|n| &n.id).collect();
    node_ids.retain(|id, _| live.contains(id));

    let mut graph = NodeGraph::new();
    for node in &payload.nodes {
        let engine_id = *node_ids.entry(node.id.clone()).or_default();
        graph.add_instance_with_id(engine_id, node.node_type.clone());

        let Some(def) = library.get_definition(&node.node_type) else {
            return Err(format!("unknown node type '{}'", node.node_type));
        };
        for input in &def.node.inputs {
            let Some(raw) = node.values.get(&input.name) else {
                continue;
            };
            if let Some(value) = coerce_input(&input.kind, raw) {
                graph
                    .set_input_value(engine_id, input.name.clone(), value)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    for edge in &payload.edges {
        let (Some(&from), Some(&to)) = (node_ids.get(&edge.from_node), node_ids.get(&edge.to_node))
        else {
            continue;
        };
        if let Err(e) = graph.connect(from, edge.from_output.clone(), to, edge.to_input.clone()) {
            util::debug_log_warning!("Skipping invalid connection: {e}");
        }
    }

    let output_node = payload
        .output_source
        .as_ref()
        .and_then(|id| node_ids.get(id).copied());

    drop(node_ids);

    // Discard frames rendered against the previous graph.
    frames.generation.fetch_add(1, Ordering::SeqCst);

    engine.send(EngineCommand::UpdateGraph(graph))?;
    engine.send(EngineCommand::SetOutputNode(output_node))
}

/// Convert a JSON widget value from the frontend into the [`InputValue`] the
/// input's definition expects. Returns `None` for kinds that have no widget
/// (frames, MIDI, port selections) or values that don't parse.
fn coerce_input(kind: &NodeInputKind, raw: &serde_json::Value) -> Option<InputValue> {
    // <select> values and number-input strings arrive as JSON strings.
    fn as_f64(value: &serde_json::Value) -> Option<f64> {
        value.as_f64().or_else(|| value.as_str()?.parse().ok())
    }

    match kind {
        NodeInputKind::Float { .. } => Some(InputValue::Float(as_f64(raw)? as f32)),
        NodeInputKind::Int { .. } => Some(InputValue::Int(as_f64(raw)? as i32)),
        NodeInputKind::Bool { .. } => Some(InputValue::Bool(raw.as_bool()?)),
        NodeInputKind::Enum { .. } => Some(InputValue::Enum(as_f64(raw)? as usize)),
        NodeInputKind::Text { .. } => Some(InputValue::Text(raw.as_str()?.to_string())),
        NodeInputKind::File { .. } => {
            let path = raw.as_str()?;
            (!path.is_empty()).then(|| InputValue::File(PathBuf::from(path)))
        }
        NodeInputKind::PortSelection => Some(InputValue::Text(raw.as_str()?.to_string())),
        _ => None,
    }
}
