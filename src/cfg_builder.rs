use petgraph::graph::{DiGraph, NodeIndex};
use rustpython_parser::ast::{Expr, Stmt, StmtFor, StmtIf, StmtWhile};
use rustpython_parser::{ast, Parse};

use crate::cfg::{CfGraph, Cfg, CfgNode, CfgNodeType};
use crate::graph_utils::{connect_children, connect_with_merge, create_and_connect};

pub struct CfgBuilder {
    ast: Vec<Stmt>,
    text: String,
    //something for for noting the entry points for functions should be here
}

#[derive(Default, Debug, Clone)]
pub struct CfgIntermediate {
    pub tail_nodes: Vec<NodeIndex>,
    pub terminate_nodes: Vec<NodeIndex>,
    pub continue_nodes: Vec<NodeIndex>,
}

impl CfgBuilder {
    pub fn new(source: String) -> Self {
        let ast = ast::Suite::parse(&source, "<embedded>").unwrap();
        // println!("{:#?}", ast);
        Self { ast, text: source }
    }

    fn is_flat(node: &Stmt) -> bool {
        matches!(
            node,
            Stmt::Assign(_)
                | Stmt::AugAssign(_)
                | Stmt::Expr(_)
                | Stmt::Pass(_)
                | Stmt::Return(_)
                | Stmt::Delete(_)
                | Stmt::Import(_)
                | Stmt::ImportFrom(_)
                | Stmt::Global(_)
                | Stmt::Nonlocal(_)
        )
    }

    fn extract_branches(stmt: StmtIf) -> (Vec<StmtIf>, Vec<Stmt>) {
        let mut branches = vec![stmt.clone()];
        let mut final_branch = vec![];

        let mut current = stmt.orelse.as_slice();

        while let Some(Stmt::If(inner_if)) = current.first() {
            branches.push(inner_if.clone());
            current = &inner_if.orelse;
        }

        if !current.is_empty() {
            final_branch.extend_from_slice(current);
        }

        (branches, final_branch)
    }

    fn accum_statement_nodes(nodes: Vec<CfgNode>) -> CfgNode {
        nodes
            .into_iter()
            .reduce(|acc, x| {
                let mut current_asn = acc.asn;
                current_asn.extend(x.asn);
                CfgNode {
                    node_type: CfgNodeType::Statement,
                    text: format!("{}\n{}", acc.text, x.text),
                    asn: current_asn,
                }
            })
            .expect("error accumulating statement nodes")
    }

    fn create_node(&self, ast: Stmt) -> CfgNode {
        let mut text = String::new();
        let node_type;
        let asn;

        match ast {
            Stmt::Assign(stmt) => {
                node_type = CfgNodeType::Statement;
                asn = vec![Stmt::Assign(stmt.clone())];
                text = self.text[stmt.range].to_string()
            }

            Stmt::AugAssign(stmt) => {
                node_type = CfgNodeType::Statement;
                asn = vec![Stmt::AugAssign(stmt.clone())];
                text = self.text[stmt.range].to_string()
            }

            Stmt::If(stmt) => {
                node_type = CfgNodeType::Condition;
                asn = vec![Stmt::If(stmt.clone())];
                match *stmt.test {
                    Expr::Compare(expr_compare) => text = self.text[expr_compare.range].to_string(),
                    _ => todo!(),
                };
                // text = self.text[stmt.range].to_string()
            }

            Stmt::While(stmt) => {
                node_type = CfgNodeType::Statement;
                asn = vec![Stmt::While(stmt.clone())];
                match *stmt.test {
                    Expr::Compare(expr_compare) => text = self.text[expr_compare.range].to_string(),
                    _ => todo!(),
                };
                //extract condition node
            }

            Stmt::For(stmt) => {
                node_type = CfgNodeType::Statement;
                asn = vec![Stmt::For(stmt.clone())];
                match *stmt.target {
                    Expr::Name(name) => text += &self.text[name.range].to_string(),
                    Expr::Tuple(tup) => text += &self.text[tup.range].to_string(),
                    _ => {}
                }

                text += " in ";

                match *stmt.iter {
                    Expr::Call(call) => text += &self.text[call.range].to_string(),
                    Expr::Name(name) => text += &self.text[name.range].to_string(),
                    _ => {}
                }
            }

            _ => {
                panic!("unsupported statement type");
                todo!()
            }
        }

        CfgNode {
            node_type,
            text,
            asn,
        }
    }

    fn part_flat(&self, ast: Vec<Stmt>) -> (CfgNode, Vec<Stmt>) {
        let mut nodes = vec![];
        let mut ast = ast.into_iter().peekable();

        while let Some(node) = ast.peek() {
            if CfgBuilder::is_flat(node) {
                nodes.push(self.create_node(ast.next().unwrap()));
            } else {
                break;
            }
        }

        (CfgBuilder::accum_statement_nodes(nodes), ast.collect())
    }

    fn merge_intermediates(intermediates: Vec<CfgIntermediate>) -> CfgIntermediate {
        let tail_nodes = intermediates
            .iter()
            .cloned()
            .map(|arm| arm.tail_nodes)
            .flatten()
            .collect::<Vec<NodeIndex>>();

        let terminate_nodes = intermediates
            .iter()
            .cloned()
            .map(|arm| arm.terminate_nodes)
            .flatten()
            .collect::<Vec<NodeIndex>>();

        let continue_nodes = intermediates
            .iter()
            .cloned()
            .map(|arm| arm.continue_nodes)
            .flatten()
            .collect::<Vec<NodeIndex>>();

        return CfgIntermediate {
            tail_nodes,
            terminate_nodes,
            continue_nodes,
        };
    }

    fn handle_if(
        &self,
        stmt: StmtIf,
        entry: NodeIndex,
        graph: &mut CfGraph,
        rem: Vec<Stmt>,
    ) -> CfgIntermediate {
        let (arms, final_branch) = CfgBuilder::extract_branches(stmt.clone());
        let mut arm_interms = vec![];

        for mut arm in arms.into_iter() {
            arm.orelse = vec![];
            let node_data = self.create_node(Stmt::If(arm.clone()));
            let arm_node = create_and_connect(graph, entry, node_data);

            //extract body
            let body = arm.body.clone();
            let arm_intermediary = self.attach_ast(arm_node, graph, body);
            arm_interms.push(arm_intermediary)
        }

        let interm = CfgBuilder::merge_intermediates(arm_interms);
        // println!("if_interm - {:?}", interm);

        if rem.is_empty() && final_branch.is_empty() {
            // No more statements to process, return the last arm nodes
            return interm;
        }

        if rem.is_empty() {
            // Only the final branch remains
            return if final_branch.is_empty() {
                return interm;
            } else {
                CfgBuilder::merge_intermediates(vec![
                    interm,
                    self.attach_ast(entry, graph, final_branch),
                ])
            };
        }

        if final_branch.is_empty() {
            // No final branch, attach the remaining statements
            let rem_interm = self.attach_ast(entry, graph, rem.to_vec());
            return CfgBuilder::merge_intermediates(vec![interm, rem_interm]);
        } else {
            // Handle both `rem` and `final_branch`
            let merge_node = graph.add_node(CfgNode::merge());

            // Connect intermediate results to the merge node
            connect_children(graph, merge_node, interm.tail_nodes.clone(), false);

            let final_branch_interm = self.attach_ast(entry, graph, final_branch);
            connect_children(
                graph,
                merge_node,
                final_branch_interm.tail_nodes.clone(),
                false,
            );

            // Process remaining statements from the merge node
            let rem_interm = self.attach_ast(merge_node, graph, rem.to_vec());

            // Merge all intermediate results
            return CfgBuilder::merge_intermediates(vec![interm, final_branch_interm, rem_interm]);
        }
    }

    fn handle_for(
        &self,
        stmt: StmtFor,
        entry: NodeIndex,
        graph: &mut CfGraph,
        rem: Vec<Stmt>,
    ) -> CfgIntermediate {
        let node_data = self.create_node(Stmt::For(stmt.clone()));
        let head = graph.add_node(node_data);
        graph.add_edge(entry, head, ());

        let body_interm = self.attach_ast(head, graph, stmt.body.clone());
        let last_node = connect_with_merge(
            graph,
            body_interm
                .tail_nodes
                .iter()
                .cloned()
                .chain(body_interm.continue_nodes.into_iter())
                .collect(),
            head,
        );

        if !(rem.is_empty()) {
            if body_interm.terminate_nodes.len() == 1 {
                let node = *body_interm.terminate_nodes.first().unwrap();
                graph.add_edge(last_node, node, ());
                self.attach_ast(node, graph, rem)
            } else if body_interm.terminate_nodes.len() > 1 {
                let merge = graph.add_node(CfgNode::merge());
                connect_children(graph, merge, body_interm.terminate_nodes, true);
                graph.add_edge(last_node, merge, ());
                self.attach_ast(merge, graph, rem)
            } else {
                self.attach_ast(head, graph, rem)
            }
        } else {
            CfgIntermediate::default()
        }
    }

    fn handle_while(
        &self,
        stmt: StmtWhile,
        entry: NodeIndex,
        graph: &mut CfGraph,
        rem: Vec<Stmt>,
    ) -> CfgIntermediate {
        let node_data = self.create_node(Stmt::While(stmt.clone()));

        let head = graph.add_node(node_data);
        graph.add_edge(entry, head, ());

        let continue_block = create_and_connect(
            graph,
            head,
            CfgNode {
                node_type: CfgNodeType::Condition,
                asn: vec![],
                text: "True".to_string(),
            },
        );

        let break_block = create_and_connect(
            graph,
            head,
            CfgNode {
                node_type: CfgNodeType::Condition,
                asn: vec![],
                text: "False".to_string(),
            },
        );

        let body_interm = self.attach_ast(continue_block, graph, stmt.body.clone());
        // println!("{:?}", body_interm);

        connect_with_merge(
            graph,
            body_interm
                .tail_nodes
                .into_iter()
                .chain(body_interm.continue_nodes.into_iter())
                .collect(),
            head,
        );

        if !body_interm.terminate_nodes.is_empty() {
            let merge = graph.add_node(CfgNode::merge());
            graph.add_edge(break_block, merge, ());

            connect_children(graph, merge, body_interm.terminate_nodes, false);

            self.attach_ast(merge, graph, rem.to_vec())
        } else {
            self.attach_ast(break_block, graph, rem.to_vec())
        }
    }

    fn attach_ast(&self, entry: NodeIndex, graph: &mut CfGraph, ast: Vec<Stmt>) -> CfgIntermediate {
        if !ast.is_empty() {
            let (first, rem) = ast.split_first().unwrap();
            if CfgBuilder::is_flat(first) {
                let (node_data, ast_rem) = self.part_flat(ast);
                let node = create_and_connect(graph, entry, node_data);
                let mut rest_interm = self.attach_ast(node, graph, ast_rem);
                if rest_interm.tail_nodes.is_empty() {
                    rest_interm.tail_nodes.push(node)
                }
                rest_interm
            } else {
                match first {
                    Stmt::If(stmt) => self.handle_if(stmt.clone(), entry, graph, rem.to_vec()),

                    Stmt::While(stmt) => {
                        self.handle_while(stmt.clone(), entry, graph, rem.to_vec())
                    }

                    Stmt::Break(_) => {
                        let mut interm = self.attach_ast(entry, graph, rem.to_vec());
                        interm.terminate_nodes.push(entry);
                        return interm;
                    }

                    Stmt::Continue(_) => {
                        let mut interm = self.attach_ast(entry, graph, rem.to_vec());
                        interm.continue_nodes.push(entry);
                        return interm;
                    }

                    Stmt::For(stmt) => self.handle_for(stmt.clone(), entry, graph, rem.to_vec()),

                    _ => {
                        // println!("{:?}", first);
                        todo!();
                    }
                }
            }
        } else {
            CfgIntermediate::default()
        }
    }

    fn build(&self, graph: &mut DiGraph<CfgNode, ()>) {
        let entry = graph.add_node(CfgNode {
            node_type: CfgNodeType::Statement,
            asn: vec![],
            text: "entry".to_string(),
        });

        let ast = &self.ast;

        self.attach_ast(entry, graph, ast.clone());
    }
}

impl From<CfgBuilder> for Cfg {
    fn from(builder: CfgBuilder) -> Self {
        let mut graph = DiGraph::new();
        builder.build(&mut graph);
        Cfg { graph }
    }
}
