use engine::node::engine_node::NumberInputUiMode;
use engine::node::{NodeInputKind, NodeLibrary, NodeOutputKind};
use media::midi::streams::list_ports;
use serde::Serialize;

#[derive(Serialize, Default)]
pub struct WidgetData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_float: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_int: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_bool: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_choice_idx: Option<usize>,
    pub use_slider: bool,
}

#[derive(Serialize)]
pub struct PortDef {
    pub id: String,
    pub label: String,
    pub port_type: String,
    pub show_pin: bool,
    pub widget: WidgetData,
}

#[derive(Serialize)]
pub struct NodeDef {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub subcategories: Vec<String>,
    pub description: String,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
}

fn input_port_type(kind: &NodeInputKind) -> &'static str {
    match kind {
        NodeInputKind::Frame => "video",
        NodeInputKind::MidiPacket => "midi",
        NodeInputKind::Bool { .. } => "bool",
        NodeInputKind::Int { .. } => "int",
        NodeInputKind::Float { .. } => "float",
        NodeInputKind::Dimensions { .. } => "dimensions",
        NodeInputKind::Pixel { .. } => "pixel",
        NodeInputKind::Enum { .. } => "enum",
        NodeInputKind::Text { .. } => "text",
        NodeInputKind::File { .. } => "file",
        NodeInputKind::PortSelection => "port_selection",
    }
}

fn output_port_type(kind: NodeOutputKind) -> &'static str {
    match kind {
        NodeOutputKind::Frame => "video",
        NodeOutputKind::MidiPacket => "midi",
        NodeOutputKind::Bool => "bool",
        NodeOutputKind::Int => "int",
        NodeOutputKind::Float => "float",
        NodeOutputKind::Dimensions => "dimensions",
        NodeOutputKind::Pixel => "pixel",
        NodeOutputKind::Text => "text",
    }
}

fn widget_for_input(kind: &NodeInputKind) -> WidgetData {
    match kind {
        NodeInputKind::Float { default, min, max, step, input_ui, .. } => WidgetData {
            default_float: Some(*default),
            min: min.map(|v| v as f64),
            max: max.map(|v| v as f64),
            step: Some(*step as f64),
            use_slider: matches!(input_ui, NumberInputUiMode::Slider),
            ..Default::default()
        },
        NodeInputKind::Int { default, min, max, step, input_ui, .. } => WidgetData {
            default_int: Some(*default),
            min: min.map(|v| v as f64),
            max: max.map(|v| v as f64),
            step: Some(*step as f64),
            use_slider: matches!(input_ui, NumberInputUiMode::Slider),
            ..Default::default()
        },
        NodeInputKind::Bool { default } => WidgetData {
            default_bool: Some(*default),
            ..Default::default()
        },
        NodeInputKind::Enum { choices, default_idx } => WidgetData {
            choices: Some(choices.clone()),
            default_choice_idx: *default_idx,
            ..Default::default()
        },
        _ => WidgetData::default(),
    }
}

#[tauri::command]
pub fn list_midi_ports() -> Vec<String> {
    list_ports()
        .map(|iter| iter.map(|p| p.port_name().to_string()).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_node_definitions() -> Vec<NodeDef> {
    let library = match NodeLibrary::load_all() {
        Ok(lib) => lib,
        Err(e) => {
            util::debug_log_warning!("Failed to load node library: {}", e);
            return Vec::new();
        }
    };

    let mut defs: Vec<NodeDef> = library
        .definitions()
        .values()
        .map(|def| {
            let node = &def.node;
            NodeDef {
                node_type: node.name.clone(),
                label: node.name.clone(),
                category: if node.category.is_empty() {
                    "Other".to_string()
                } else {
                    node.category.clone()
                },
                subcategories: node.subcategories.clone(),
                description: node.short_description.clone(),
                inputs: node
                    .inputs
                    .iter()
                    .map(|p| PortDef {
                        id: p.name.clone(),
                        label: p.name.clone(),
                        port_type: input_port_type(&p.kind).to_string(),
                        show_pin: p.show_pin,
                        widget: widget_for_input(&p.kind),
                    })
                    .collect(),
                outputs: node
                    .outputs
                    .iter()
                    .map(|p| PortDef {
                        id: p.name.clone(),
                        label: p.name.clone(),
                        port_type: output_port_type(p.kind).to_string(),
                        show_pin: p.show_pin,
                        widget: WidgetData::default(),
                    })
                    .collect(),
            }
        })
        .collect();

    defs.sort_by(|a, b| a.label.cmp(&b.label));
    defs
}
