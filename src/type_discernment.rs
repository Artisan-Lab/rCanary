use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::context::RlcCtxt;
use crate::{rlc_info};

pub mod init;
pub mod visitor;
pub mod solver;

type Ty2Sting<'tcx> = FxHashMap<Ty<'tcx>, String>;
type Sting2Ty<'tcx> = FxHashMap<String, Ty<'tcx>>;
type LlvmResSet = FxHashSet<String>;
type FuncSet = FxHashSet<DefId>;

// Type Discernment is the first phase for ATC Analysis and it will perform a simple-inter-procedural analysis
// for current crate that can grasp all types after monomorphism of generics.
// The struct TypeCollector impls mir::Visitor to perform this analysis to construct all dependency of types.
// Note: the type in this phase is Ty::ty instead of Hir::ty.
pub struct TypeDiscernment<'tcx> {
    rcx: RlcCtxt<'tcx>,
    llvm_res_set: LlvmResSet,
    type_map: TypeMap<'tcx>,
    func_set: FuncSet,
}

impl<'tcx> TypeDiscernment<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>) -> Self {
        Self {
            rcx,
            llvm_res_set: FxHashSet::default(),
            type_map: TypeMap::default(),
            func_set: FxHashSet::default(),
        }
    }

    pub fn rcx(&self) -> RlcCtxt<'tcx> {
        self.rcx
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.rcx().tcx()
    }

    pub fn llvm_res_set(&self) -> &LlvmResSet {
        &self.llvm_res_set
    }

    pub fn llvm_res_set_mut(&mut self) -> &mut LlvmResSet {
        &mut self.llvm_res_set
    }

    pub fn type_map(&self) -> &TypeMap<'tcx> {
        &self.type_map
    }

    pub fn type_map_mut(&mut self) -> &mut TypeMap<'tcx> {
        &mut self.type_map
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

        // Get the analysis result from rlc phase llvm
        self.init();
        // Get related adt types through visiting mir local
        self.visitor();
        // Solving types by local ty and rlc llvm result
        self.solver();

    }
}

#[derive(Debug, Clone)]
pub struct TypeMap<'tcx> {
    ty_to_string: Ty2Sting<'tcx>,
    string_to_ty: Sting2Ty<'tcx>,
}

impl<'tcx> Default for TypeMap<'tcx> {
    fn default() -> Self {
        Self {
            ty_to_string: FxHashMap::default(),
            string_to_ty: FxHashMap::default(),
        }
    }
}

impl<'tcx> TypeMap<'tcx> {
    fn get_t2s(&self) -> &Ty2Sting<'tcx> {
        &self.ty_to_string
    }

    fn get_s2t(&self) -> &Sting2Ty<'tcx> {
        &self.string_to_ty
    }

    fn insert_ty(&mut self, ty: Ty<'tcx>) {
        let s = format!("{:?}", ty);
        self.string_to_ty.insert(s.clone(), ty);
        self.ty_to_string.insert(ty, s);
    }

    fn remove_ty(&mut self, ty: Ty<'tcx>) {
        let s = format!("{:?}", ty);
        self.string_to_ty.remove(&s);
        self.ty_to_string.remove(ty);
    }


    fn remove_string(&mut self, s: String) {
        let ty = self.string_to_ty.get(&s).unwrap().clone();
        self.string_to_ty.remove(&s);
        self.ty_to_string.remove(&ty);
    }
}