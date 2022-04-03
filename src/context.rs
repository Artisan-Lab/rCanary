use rustc_middle::ty::TyCtxt;

use std::collections::HashMap;

use crate::RlcConfig;
use crate::type_analysis::AdtOwner;
use crate::flow_analysis::MirGraph;

#[derive(Clone)]
pub struct RlcGlobalCtxt<'tcx> {
    tcx: TyCtxt<'tcx>,
    config: RlcConfig,
    adt_owner: AdtOwner,
    mir_graph: MirGraph,
}

impl<'tcx> RlcGlobalCtxt<'tcx> {
    pub fn new(tcx:TyCtxt<'tcx>, config: RlcConfig) -> Self {
        Self {
            tcx,
            config,
            adt_owner: HashMap::default(),
            mir_graph: HashMap::default(),
        }
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    pub fn config(&self) -> RlcConfig {
        self.config
    }

    pub fn adt_owner(&self) -> &AdtOwner {
        &self.adt_owner
    }

    pub fn adt_owner_mut(&mut self) -> &mut AdtOwner {
        &mut self.adt_owner
    }

    pub fn mir_graph(&self) -> &MirGraph {
        &self.mir_graph
    }

    pub fn mir_graph_mut(&mut self) -> &mut MirGraph {
        &mut self.mir_graph
    }
}