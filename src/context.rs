use rustc_middle::ty::TyCtxt;
use crate::RlcConfig;


pub struct RlcGlobalCtxt<'tcx> {
    tcx: TyCtxt<'tcx>,
    config: RlcConfig,
}

impl<'tcx> RlcGlobalCtxt<'tcx> {
    pub fn new(tcx:TyCtxt<'tcx>, config: RlcConfig) -> Self {
        Self {
            tcx,
            config,
        }
    }
}

pub type RlcCtxt<'tcx> = &'tcx RlcGlobalCtxt<'tcx>;


