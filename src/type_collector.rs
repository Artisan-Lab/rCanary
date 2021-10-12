use rustc_data_structures::fx::FxHashMap;

use crate::context::RlcCtxt;

pub struct TypeCollector<'tcx> {
    rcx: RlcCtxt<'tcx>,


}

impl<'tcx> TypeCollector<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>) -> Self {
        Self {
            rcx
        }
    }

    pub fn start(&mut self) {

    }
}