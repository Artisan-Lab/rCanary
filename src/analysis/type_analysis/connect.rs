use crate::RLC_LLVM_IR;
use crate::analysis::type_analysis::TypeAnalysis;
use crate::components::fs::{rlc_can_read_dir, rlc_read, rlc_demangle, rlc_create_file};

use std::io::{BufRead, BufReader, Write};
use std::collections::{HashMap, HashSet};

use serde_json::json;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CallGraph {
    g: Graph,
}

type Graph = HashMap<String, HashSet<String>>;

impl Default for CallGraph {
    fn default() -> Self {
        Self { g: HashMap::default() }
    }
}

impl CallGraph {
    pub fn graph(&self) -> &Graph {
        &self.g
    }

    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.g
    }
}

impl<'tcx, 'a> TypeAnalysis<'tcx, 'a> {
    pub fn connect(&mut self) {
        if rlc_can_read_dir(RLC_LLVM_IR, "Cannot read LLVM IR files") {
            let mut call_graph = CallGraph::default();
            for entry in WalkDir::new(RLC_LLVM_IR) {
                let entry_path = entry.unwrap().into_path();
                if entry_path
                    .iter()
                    .last()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .ends_with(".rlc")
                {
                    let file = rlc_read(entry_path,"Failed to read file");
                    let fin = BufReader::new(file);

                    let mut caller = String::new();
                    for line in fin.lines() {
                        let s = line.unwrap();
                        if !s.starts_with("     ") {
                            caller = s;
                            call_graph
                                .graph_mut()
                                .insert(rlc_demangle(&caller), HashSet::new());
                        } else {
                            let callee = s.replace("     ", "");
                            call_graph
                                .graph_mut()
                                .get_mut(&rlc_demangle(&caller))
                                .unwrap()
                                .insert(rlc_demangle(&callee));
                        }
                    }

                }
            }
            let json_value = json!(call_graph);
            let mut file = rlc_create_file("/tmp/rlc/cg.json", "failed to create call graph (json)");
            file.write(json_value.to_string().as_bytes());
        }
    }
}