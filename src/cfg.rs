//=todo= adding final cases to if statements
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use rustpython_parser::ast::Stmt;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub type CfGraph = DiGraph<CfgNode, ()>;

pub enum CfgNodeType {
    Statement,
    Condition,
    Merge,
}

pub struct CfgNode {
    pub node_type: CfgNodeType,
    pub text: String,
    //abstract syntax node
    pub asn: Vec<Stmt>,
    //counter to assign node ids
}

impl CfgNode {
    pub fn merge() -> Self {
        CfgNode {
            node_type: CfgNodeType::Merge,
            text: "merge".to_string(),
            asn: vec![],
        }
    }

    fn if_merge() -> Self {
        CfgNode {
            node_type: CfgNodeType::Merge,
            text: "if merge".to_string(),
            asn: vec![],
        }
    }
}

impl fmt::Debug for CfgNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

pub struct CfgConnection {
    pub node: CfgNode,
    pub to: usize,
}

pub struct Cfg {
    pub graph: DiGraph<CfgNode, ()>,
}

impl fmt::Display for Cfg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[Config::EdgeNoLabel],
                &|_, _| "".to_string(),
                &|_, (_, node)| {
                    match node.node_type {
                        CfgNodeType::Condition => r#"shape="box""#.to_string(),
                        _ => "".to_string(),
                    }
                }
            )
        )
    }
}

impl Cfg {
    pub fn to_img(&self, output_path: &str) {
        // Wrap the graph in the Cfg struct to match your Display impl

        // Generate the DOT string using the Display implementation
        let dot_content = format!("{}", self);

        // Create a temporary DOT file
        let dot_file_path = format!("{}.dot", output_path);
        let mut dot_file = File::create(&dot_file_path).expect("Unable to create DOT file");
        dot_file
            .write_all(dot_content.as_bytes())
            .expect("Unable to write to DOT file");

        // Use `dot` command to generate PNG
        let output = Command::new("dot")
            .args(&["-Tpng", &dot_file_path, "-o", output_path])
            .output()
            .expect("Failed to execute Graphviz `dot` command");

        if !output.status.success() {
            eprintln!(
                "Graphviz error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            std::fs::remove_file(&dot_file_path).ok(); // Clean up temporary file
            panic!("Failed to generate PNG from DOT graph");
        }

        // Remove the temporary DOT file
        std::fs::remove_file(&dot_file_path).expect("Unable to delete temporary DOT file");
    }
}
