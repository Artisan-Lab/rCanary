use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::{self,TyCtxt, Ty};
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{self};
use rustc_middle::mir::BasicBlock;
use rustc_span::def_id::DefId;

use crate::context::RlcCtxt;
use crate::display::Display;
use crate::rlc_info;

// Type Collector is the first phase for ATC Analysis and it will perform a simple-inter-procedural analysis
// for current crate that can grasp all types after monomorphism of generics.
// The struct TypeCollector impls mir::Visitor to perform this analysis to construct all dependency of types.
// Note: the type in this phase is Ty::ty instead of Hir::ty.
pub struct TypeCollector<'tcx> {
    rcx: RlcCtxt<'tcx>,
    type_set: TySet<'tcx>,
}

type TySet<'tcx> = FxHashSet<Ty<'tcx>>;

impl<'tcx> Visitor<'tcx> for TypeCollector<'tcx> {
    fn visit_body(&mut self, body: &mir::Body<'tcx>) {
        println!("{}", body.display());
    }
}

impl<'tcx> TypeCollector<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>) -> Self {
        Self {
            rcx,
            type_set: FxHashSet::default(),
        }
    }

    pub fn rcx(&self) -> RlcCtxt<'tcx> {
        self.rcx
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.rcx().tcx()
    }

    pub fn type_set(&mut self) -> &TySet<'tcx> {
        &self.type_set
    }

    // The main phase and the starter function of Type Collector.
    // RLC will construct an instance of struct TypeCollector and call self.start to make analysis starting.
    pub fn start(&mut self) {

        // Get the Global TyCtxt from rustc
        let tcx = self.tcx();

        //tcx.hir().krate().visit_all_item_likes(self);

        // Grasp all mir Keys defined in current crate
        let mir_keys = tcx.mir_keys(());
        for each_mir in mir_keys {
            let body = mir_body(tcx, each_mir.to_def_id());
            self.visit_body(body);
        }
    }
}

fn mir_body<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &'tcx mir::Body<'tcx> {
    let id = ty::WithOptConstParam::unknown(def_id);
    let def = ty::InstanceDef::Item(id);
    tcx.instance_mir(def)
}

// An implementation for ItemLikeVisitor in HIR
// impl<'tcx> ItemLikeVisitor<'tcx> for TypeCollector<'tcx> {
//     fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
//         //rlc_info!("{:?}", item);
//     }
//
//     fn visit_trait_item(&mut self, _trait_item: &'tcx rustc_hir::TraitItem<'tcx>) {
//     }
//
//     fn visit_impl_item(&mut self, _impl_item: &'tcx rustc_hir::ImplItem<'tcx>) {
//     }
//
//     fn visit_foreign_item(&mut self, foreign_item: &'tcx rustc_hir::ForeignItem<'tcx>) {
//         rlc_info!("{:?}", foreign_item);
//     }
// }