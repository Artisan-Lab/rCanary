pub mod type_analysis;
pub mod flow_analysis;

use rustc_middle::ty::TyCtxt;

use crate::components::context::RlcGlobalCtxt;
use crate::analysis::flow_analysis::{IcxSliceFroBlock, IntroFlowContext};

pub trait Tcx<'tcx, 'o, 'a> {
    fn tcx(&'o self) -> TyCtxt<'tcx>;
}

pub trait Rcx<'tcx, 'o, 'a> {
    fn rcx(&'o self) -> &'a RlcGlobalCtxt<'tcx>;

    fn tcx(&'o self) -> TyCtxt<'tcx>;
}

pub trait RcxMut<'tcx, 'o, 'a> {
    fn rcx(&'o self) -> &'o RlcGlobalCtxt<'tcx>;

    fn rcx_mut(&'o mut self) -> &'o mut RlcGlobalCtxt<'tcx>;

    fn tcx(&'o self) -> TyCtxt<'tcx>;
}

pub trait IcxMut<'tcx, 'ctx, 'o> {
    fn icx(&'o self) -> &'o IntroFlowContext<'tcx, 'ctx>;

    fn icx_mut(&'o mut self) -> &'o mut IntroFlowContext<'tcx, 'ctx>;
}

pub trait IcxSliceMut<'tcx, 'ctx, 'o> {
    fn icx_slice(&'o self) -> &'o IcxSliceFroBlock<'tcx, 'ctx>;

    fn icx_slice_mut(&'o mut self) -> &'o mut IcxSliceFroBlock<'tcx, 'ctx>;
}