use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::TyCtxt;
use rustc_hir::{Item, itemlikevisit::ItemLikeVisitor};

use crate::context::RlcCtxt;
use crate::rlc_info;

type TySet<'tcx> = FxHashSet<Item<'tcx>>;

pub struct TypeCollector<'tcx> {
    rcx: RlcCtxt<'tcx>,
    type_items: TySet<'tcx>,
}

impl<'tcx> ItemLikeVisitor<'tcx> for TypeCollector<'tcx> {
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
        //rlc_info!("{:?}", item);
    }

    fn visit_trait_item(&mut self, _trait_item: &'tcx rustc_hir::TraitItem<'tcx>) {
    }

    fn visit_impl_item(&mut self, _impl_item: &'tcx rustc_hir::ImplItem<'tcx>) {
    }

    fn visit_foreign_item(&mut self, foreign_item: &'tcx rustc_hir::ForeignItem<'tcx>) {
        rlc_info!("{:?}", foreign_item);
    }
}

impl<'tcx> TypeCollector<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>) -> Self {
        Self {
            rcx,
            type_items: FxHashSet::default(),
        }
    }

    pub fn rcx(&self) -> RlcCtxt<'tcx> {
        self.rcx
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.rcx().tcx()
    }

    pub fn start(&mut self) {
        let tcx = self.tcx();
        //println!("{:?}", tcx.lang_items());
        tcx.hir().krate().visit_all_item_likes(self);
        let mir_keys = tcx.mir_keys(());
        for each_mir in mir_keys {
            println!("{:?}", each_mir);
            if tcx.is_optimized_mir(each_mir) {
                println!("{:?}",tcx.optimized_mir(*each_mir));
            } else {

            }
        }
    }
}