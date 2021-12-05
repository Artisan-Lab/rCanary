use super::TypeAnalysis;
use crate::log::rlc_error_and_exit;
use crate::RLC_LLVM_IR;
use crate::fs::{rlc_can_read_dir, rlc_read, rlc_demangle};

use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};

use walkdir::WalkDir;

type CallGraph = HashMap<String, HashSet<String>>;

impl<'tcx> TypeAnalysis<'tcx> {
    pub fn init(&mut self) {
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
                    let mut fin = BufReader::new(file);

                    let mut caller = String::new();
                    for line in fin.lines() {
                        let s = line.unwrap();
                        if !s.starts_with("     ") {
                            caller = s;
                            call_graph
                                .insert(rlc_demangle(&caller), HashSet::new());
                        } else {
                            let callee = s.replace("     ", "");
                            call_graph
                                .get_mut(&rlc_demangle(&caller))
                                .unwrap()
                                .insert(rlc_demangle(&callee));
                        }
                    }

                }
            }
            for elem in call_graph {
                // println!("{}",elem.0);
            }
        }
    }
}