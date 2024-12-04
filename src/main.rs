use crate::cfg::Cfg;
use cfg_builder::CfgBuilder;
use std::fs;

mod cfg;
mod cfg_builder;
mod graph_utils;
// mod graph;

fn main() {
    let python_source = fs::read_to_string("test/simple.py").expect("Unable to read file");
    let builder = CfgBuilder::new(python_source);
    let cfg = Cfg::from(builder);
    cfg.to_img("graph.png");
    println!("{}", cfg);
}
