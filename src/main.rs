use eframe::{App, CreationContext, NativeOptions, run_native};
use egui::{Color32, Pos2, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_graphs::{
    DefaultGraphView, FruchtermanReingoldState, FruchtermanReingoldWithCenterGravityState, Graph,
    SettingsInteraction, SettingsNavigation, SettingsStyle, to_graph,
};
use petgraph::{
    Direction::Incoming,
    Undirected,
    graph::{EdgeIndex, Edges, NodeIndex, UnGraph},
    prelude::StableUnGraph,
    stable_graph::StableGraph,
    visit::EdgeRef,
};
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    process::{Command, Stdio},
    sync::mpsc::{Receiver, Sender},
    time::Duration,
}; //, self, Write};
//use std::str;
use notify::{RecommendedWatcher, RecursiveMode, event::EventKindMask};
use notify_debouncer_mini::{Config, DebounceEventResult, DebouncedEvent, new_debouncer_opt};
use std::path::Path;

pub struct BasicApp {
    g: Graph<Note, (), Undirected>,
    nodes: HashMap<String, NodeIndex>,
    links: HashSet<String>,
    rx: Receiver<MessageType>,
}

#[derive(Debug)]
pub struct MessageType {
    msg_type: String,
    metadata: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    filename: String,
    #[serde(rename = "filenameStem")]
    filename_stem: String,
    path: String,
    #[serde(rename = "absPath")]
    abs_path: String,
    title: String,
    link: String,
    lead: String,
    body: String,
    snippets: Vec<String>,
    #[serde(rename = "rawContent")]
    raw_content: String,
    #[serde(rename = "wordCount")]
    word_count: usize,
    tags: Vec<String>,
    metadata: Value,
    created: String,
    modified: String,
    checksum: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Link {
    title: String,
    href: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "isExternal")]
    is_external: bool,
    rels: Vec<String>,
    snippet: String,
    #[serde(rename = "snippetStart")]
    snippet_start: usize,
    #[serde(rename = "snippetEnd")]
    snippet_end: usize,
    #[serde(rename = "sourceId")]
    source_id: usize,
    #[serde(rename = "sourcePath")]
    source_path: String,
    #[serde(rename = "targetId")]
    target_id: usize,
    #[serde(rename = "targetPath")]
    target_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ZkGraph {
    notes: Vec<Note>,
    links: Vec<Link>,
}

impl BasicApp {
    fn new(_: &CreationContext<'_>, rx: Receiver<MessageType>) -> Self {
        let g: StableUnGraph<Note, ()> = StableUnGraph::default();
        let ui_graph = to_graph(&g);
        let nodes = HashMap::new();
        let links = HashSet::new();
        Self {
            g: ui_graph,
            nodes,
            links,
            rx,
        }
    }
}

impl App for BasicApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        /*
        //File watcher code
        while let Ok(Ok(payload)) = self.rx.try_recv() {
            println!("File change {:?}", payload[0]);
            //self.g.add_node_with_label((), "TEST".to_string()); //= generate_graph(render_graph());
            //self.g = generate_graph(render_graph());
        }*/

        while let Ok(payload) = self.rx.try_recv() {
            println!("RECEIEVED MSG: {:?}", payload.msg_type);
            match payload.msg_type.as_str() {
                "INS_NODE" => {
                    let note: Note = serde_json::from_value(payload.metadata).unwrap();
                    if !self.nodes.contains_key(&note.filename) {
                        let node_idx = self.g.add_node_with_label(note.clone(), note.title.clone());
                        self.nodes.insert(note.filename.clone(), node_idx);
                    } else {
                        println!("Duplicate key found: {:?}", note.filename);
                    }
                }
                "INS_EDGE" => {
                    let link: Link = serde_json::from_value(payload.metadata).unwrap();
                    if !self.links.contains(&format!(
                        "{target}-{source}",
                        target = &link.target_path,
                        source = &link.source_path
                    )) && !self.links.contains(&format!(
                        "{source}-{target}",
                        target = &link.target_path,
                        source = &link.source_path
                    )) {
                        self.links.insert(format!(
                            "{target}-{source}",
                            target = &link.target_path,
                            source = &link.source_path
                        ));
                        let source_node = self.nodes[&link.source_path];
                        let target_node = self.nodes[&link.target_path];
                        self.g.add_edge(source_node, target_node, ());
                    }

                    //ui_graph.add_edge(source_node, target_node, ());
                }
                _ => println!("Undetermined `msg_type` value"),
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            type L =
                egui_graphs::LayoutForceDirected<egui_graphs::FruchtermanReingoldWithCenterGravity>;
            type S = egui_graphs::FruchtermanReingoldWithCenterGravityState;

            let settings_interaction = SettingsInteraction::new().with_node_selection_enabled(true);
            let settings_navigation = SettingsNavigation::new().with_zoom_and_pan_enabled(true);
            //.with_fit_to_screen_enabled(false);

            let selected_nodes: Vec<_> = self.g.selected_nodes().iter().copied().collect();
            let all_nodes: Vec<_> = self.g.nodes_iter().map(|(idx, _)| idx).collect();

            //reset prev selected nodes
            for idx in all_nodes {
                self.g.node_mut(idx).unwrap().set_color(Color32::GRAY);
            }

            for idx in selected_nodes {
                let node = self.g.node_mut(idx).unwrap();
                node.set_color(Color32::from_hex("#7852EE").unwrap());

                let selected_edges = self
                    .g
                    .edges_directed(idx, Incoming)
                    .map(|edge_ref| edge_ref.id())
                    .collect();

                //println!("Selected Edges {:?}", selected_edges);

                self.g.set_selected_edges(selected_edges); //Currently does not work

                let new_se = Vec::from(self.g.selected_edges());

                //println!("NEW SE {:?}", new_se);

                let node = self.g.node(idx).unwrap();

                let default_win_pos: Pos2 = Pos2 { x: 0.0, y: 0.0 };

                egui::Window::new(node.label())
                    .default_pos(default_win_pos)
                    .scroll([false, true])
                    .show(&ctx, |ui| {
                        let payload = node.payload().clone();

                        ui.label(payload.filename);
                        ui.label(&format!(
                            "Word Count: {word_count}",
                            word_count = payload.word_count
                        ));

                        let mut cache = CommonMarkCache::default();
                        CommonMarkViewer::new().show(ui, &mut cache, payload.raw_content.as_str());
                    });
            }

            let selected_edges = Vec::from(self.g.selected_edges());

            let settings_style = SettingsStyle::new().with_edge_stroke_hook(
                move |selected, order, current_stroke, egui_style| {
                    let mut new_stroke = current_stroke.clone();

                    new_stroke.color = Color32::DARK_GRAY;
                    new_stroke.width = 0.5;

                    new_stroke
                },
            );

            let mut state: FruchtermanReingoldWithCenterGravityState =
                egui_graphs::get_layout_state(ui, None);

            state.base.c_repulse = 0.01;
            //state.base.c_attract = 0.9;

            //println!("{}", &format!("{c}", c = state.extras.0.params.c));

            //state.extras.0.params.c = 0.

            //println!("{:?}", state.base);

            egui_graphs::set_layout_state(ui, state, None);

            let mut view = egui_graphs::GraphView::<_, _, _, _, _, _, S, L>::new(&mut self.g)
                .with_styles(&settings_style)
                .with_navigations(&settings_navigation)
                .with_interactions(&settings_interaction);

            ui.add(&mut view);
        });
    }
}

fn generate_graph(tx: Sender<MessageType>, zk_graph: ZkGraph) {
    for note in zk_graph.notes.iter() {
        // SEND NEW NODE MESSAGE
        tx.send(MessageType {
            msg_type: String::from("INS_NODE"),
            metadata: serde_json::to_value(note).unwrap(),
        })
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        //nodes.insert(note.filename.clone(), g.add_node(note.clone()));
        //ui_graph.add_node_with_label(note.clone(), note.title.clone());
    }

    for link in zk_graph.links.iter() {
        tx.send(MessageType {
            msg_type: String::from("INS_EDGE"),
            metadata: serde_json::to_value(link).unwrap(),
        })
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        //g.add_edge(source_node, target_node, ());
        //ui_graph.add_edge(source_node, target_node, ());
    }
}

fn render_graph() -> ZkGraph {
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
        Err(_) => panic!("Could not read stdout of `zk graph`"),
    };

    let v: Result<ZkGraph> = serde_json::from_str(&buf);

    let result = match v {
        Ok(val) => val,
        Err(e) => panic!("Error: {e:?}. Stdout: {buf:?}"),
    };

    result
}

fn file_watcher(
    tx: Sender<MessageType>,
    file_rx: &Receiver<std::result::Result<Vec<DebouncedEvent>, notify::Error>>,
) {
    while let Ok(Ok(payload)) = file_rx.try_recv() {
        println!("Received File watcher msg: {:?}", payload);
        generate_graph(tx.clone(), render_graph());
    }
}

fn main() {
    //Create the async channel via mspc
    let (tx, rx) = std::sync::mpsc::channel::<MessageType>();

    let (file_tx, file_rx) = std::sync::mpsc::channel();

    let notify_config = notify::Config::default().with_compare_contents(true);

    let debouncer_config = Config::default()
        .with_timeout(Duration::from_secs(1))
        .with_notify_config(notify_config)
        .with_batch_mode(true);

    let mut debouncer =
        new_debouncer_opt::<_, RecommendedWatcher>(debouncer_config, file_tx).unwrap();

    debouncer
        .watcher()
        .watch(
            Path::new("C:\\Users\\Owner\\Documents\\zk\\zk\\"),
            RecursiveMode::Recursive,
        )
        .unwrap();

    std::thread::spawn(move || {
        let new_tx = tx.clone();
        println!("Thread Started. Sleeping");
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("Thread Done Sleeping");
        generate_graph(new_tx, render_graph());
        println!("Thread Generated Graph");
        println!("Starting file watcher loop");
        loop {
            file_watcher(tx.clone(), &file_rx);
        }
    });

    run_native(
        "",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(BasicApp::new(cc, rx)))),
    )
    .unwrap();
}
