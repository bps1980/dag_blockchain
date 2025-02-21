extern crate petgraph;
use petgraph::dot::{Dot, Config};
use petgraph::graph::{DiGraph, NodeIndex};

impl DAG {
    fn visualize(&self) {
        let mut graph = DiGraph::new();

        let mut node_map = HashMap::new();
        for (id, tx) in &self.transactions {
            let node = graph.add_node(id.clone());
            node_map.insert(id.clone(), node);
        }

        for (id, tx) in &self.transactions {
            for parent_id in &tx.parents {
                if let Some(&parent_node) = node_map.get(parent_id) {
                    if let Some(&current_node) = node_map.get(id) {
                        graph.add_edge(parent_node, current_node, ());
                    }
                }
            }
        }

        println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));
    }
}

fn main() {
    // Generate DAG and visualize
    let mut dag = DAG::new();
    // ... (add transactions as above)
    dag.visualize();
}
