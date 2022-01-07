use rustc_middle::ty::TyCtxt;

use crate::RlcConfig;
use crate::type_analysis::AdtOwner;

use std::collections::HashMap;

#[derive(Clone)]
pub struct RlcGlobalCtxt<'tcx> {
    tcx: TyCtxt<'tcx>,
    config: RlcConfig,
    adt_owner: AdtOwner,
}

impl<'tcx> RlcGlobalCtxt<'tcx> {
    pub fn new(tcx:TyCtxt<'tcx>, config: RlcConfig) -> Self {
        Self {
            tcx,
            config,
            adt_owner: HashMap::default(),
        }
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    pub fn adt_owner(&self) -> &AdtOwner {
        &self.adt_owner
    }

    pub fn adt_owner_mut(&mut self) -> &mut AdtOwner {
        &mut self.adt_owner
    }
}