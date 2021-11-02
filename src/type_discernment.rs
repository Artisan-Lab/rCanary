use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::{self,TyCtxt, Ty};
use rustc_middle::mir::visit::{Visitor, TyContext};
use rustc_middle::mir::{self, Body, BasicBlock, BasicBlockData, Local, LocalDecl, Operand};
use rustc_middle::mir::terminator::TerminatorKind;
use rustc_span::def_id::DefId;

use crate::context::RlcCtxt;
use crate::display::{self, Display};
use crate::{rlc_info, RlcConfig};

// Type Collector is the first phase for ATC Analysis and it will perform a simple-inter-procedural analysis
// for current crate that can grasp all types after monomorphism of generics.
// The struct TypeCollector impls mir::Visitor to perform this analysis to construct all dependency of types.
// Note: the type in this phase is Ty::ty instead of Hir::ty.
pub struct TypeDiscernment<'tcx> {
    rcx: RlcCtxt<'tcx>,
    config: RlcConfig,
    type_set: TySet<'tcx>,
    func_set: FuncSet,
}

type TySet<'tcx> = FxHashSet<Ty<'tcx>>;
type FuncSet = FxHashSet<DefId>;

impl<'tcx> Visitor<'tcx> for TypeDiscernment<'tcx> {
    fn visit_body(&mut self, body: &Body<'tcx>) {

        // Display the mir body if is Display MIR Verbose / Very Verbose
        if display::is_display_verbose() {
            println!("{}", body.display());
        }

        for (local, local_decl) in body.local_decls.iter().enumerate() {
            self.visit_local_decl(Local::from(local), local_decl);
        }

        for (block, data) in body.basic_blocks().iter().enumerate() {
            self.visit_basic_block_data(BasicBlock::from(block), data);
        }

    }


    fn visit_local_decl(&mut self, local: Local, local_decl: &LocalDecl<'tcx>) {
        let ty_context = TyContext::LocalDecl{local, source_info: local_decl.source_info};
        self.visit_ty(local_decl.ty, ty_context);
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>, ty_context: TyContext) {
        if self.type_set_mut().insert(ty) {

        }
    }

    fn visit_basic_block_data(
        &mut self,
        _block: BasicBlock,
        data: &BasicBlockData<'tcx>
    ) {
        let term = data.terminator();
        match term.kind {
            TerminatorKind::Call { ref func, args:_, destination:_, cleanup:_, from_hir_call:_, fn_span:_ } => {
                match func {
                    Operand::Constant(ref constant) => {
                        match constant.literal.ty().kind() {
                            ty::FnDef(def_id, _) => {
                                // let body = mir_body(self.tcx(), *def_id);
                                // self.visit_body(body);
                                if self.tcx().is_mir_available(*def_id) {

                                } else {
                                    println!("{:?}", constant);
                                }
                            },
                            _ => (),
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }


}

impl<'tcx> TypeDiscernment<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>, config: RlcConfig) -> Self {
        Self {
            rcx,
            config,
            type_set: FxHashSet::default(),
            func_set: FxHashSet::default(),
        }
    }

    pub fn rcx(&self) -> RlcCtxt<'tcx> {
        self.rcx
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.rcx().tcx()
    }

    pub fn type_set(&self) -> &TySet<'tcx> {
        &self.type_set
    }

    pub fn type_set_mut(&mut self) -> &mut TySet<'tcx> {
        &mut self.type_set
    }

    pub fn func_set(&self) -> &FuncSet {
        &self.func_set
    }

    pub fn func_set_mut(&mut self) -> &mut FuncSet {
        &mut self.func_set
    }

    // The main phase and the starter function of Type Collector.
    // RLC will construct an instance of struct TypeCollector and call self.start to make analysis starting.
    pub fn start(&mut self) {

        // Get the Global TyCtxt from rustc
        // Grasp all mir Keys defined in current crate
        let tcx = self.tcx();
        let mir_keys = tcx.mir_keys(());

        for each_mir in mir_keys {
            // Get the defid of current crate and get mir Body through this id
            let def_id = each_mir.to_def_id();
            let body = mir_body(tcx, def_id);

            // Insert the defid to hashset if is not existed and visit the body
            if self.func_set_mut().insert(def_id) {
                self.visit_body(body);
            } else {
                continue;
            }

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