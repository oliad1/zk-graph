use eframe::{App, CreationContext, NativeOptions, run_native};
use egui_graphs::{DefaultGraphView, Graph, to_graph};
use petgraph::{graph::NodeIndex, stable_graph::StableGraph};
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use std::{
    collections::HashMap,
    io::Read,
    process::{Command, Stdio},
}; //, self, Write};
//use std::str;

pub struct BasicApp {
    g: Graph,
}

#[derive(Serialize, Deserialize)]
pub struct Note {
    filename: String,
    filenameStem: String,
    path: String,
    absPath: String,
    title: String,
    link: String,
    lead: String,
    body: String,
    snippets: Vec<String>,
    rawContent: String,
    wordCount: u8,
    tags: Vec<String>,
    metadata: Value,
    created: String,
    modified: String,
    checksum: String,
}

#[derive(Serialize, Deserialize)]
pub struct Link {
    title: String,
    href: String,
    #[serde(rename = "type")]
    kind: String,
    isExternal: bool,
    rels: Vec<String>,
    snippet: String,
    snippetStart: u8,
    snippetEnd: u8,
    sourceId: u8,
    sourcePath: String,
    targetId: u8,
    targetPath: String,
}

#[derive(Serialize, Deserialize)]
struct ZkGraph {
    notes: Vec<Note>,
    links: Vec<Link>,
}

impl BasicApp {
    fn new(_: &CreationContext<'_>, zk_graph: ZkGraph) -> Self {
        let g = generate_graph(zk_graph);
        Self { g }
    }
}

impl App for BasicApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            type L =
                egui_graphs::LayoutForceDirected<egui_graphs::FruchtermanReingoldWithCenterGravity>;
            type S = egui_graphs::FruchtermanReingoldWithCenterGravityState;
            let mut view = egui_graphs::GraphView::<_, _, _, _, _, _, S, L>::new(&mut self.g);
            ui.add(&mut view);
        });
    }
}

fn generate_graph(zk_graph: ZkGraph) -> Graph<(), ()> {
    let mut g = StableGraph::new();
    let mut ui_graph = to_graph(&g);

    let mut nodes: HashMap<String, NodeIndex> = HashMap::new();

    for note in zk_graph.notes.iter() {
        nodes.insert(note.filename.clone(), g.add_node(()));
        ui_graph.add_node_with_label((), note.title.clone());
    }

    for link in zk_graph.links.iter() {
        let source_node = nodes[&link.sourcePath];
        let target_node = nodes[&link.targetPath];
        g.add_edge(source_node, target_node, ());
        ui_graph.add_edge(source_node, target_node, ());
    }

    ui_graph
}

fn main() {
    let child = Command::new("zk")
        .arg("graph")
        .arg("--format=json")
        .arg("--notebook-dir=C:/Users/Owner/Documents/zk/zk/")
        .stdout(Stdio::piped())
        .spawn()
        .expect("should be able to execute `zk graph`");

    let mut child_stdout = match child.stdout {
        Some(val) => val,
        None => panic!("No stdout returned by `zk graph`"),
    };

    let mut buf = String::new();

    let byte_size = match child_stdout.read_to_string(&mut buf) {
        Ok(val) => val,
        Err(_) => panic!("help me"),
    };

    let v: Result<ZkGraph> = serde_json::from_str(&buf);

    let result = match v {
        Ok(val) => val,
        Err(_) => panic!("Something bad happened"),
    };

    run_native(
        "egui_graphs_basic_demo",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(BasicApp::new(cc, result)))),
    )
    .unwrap();
}
