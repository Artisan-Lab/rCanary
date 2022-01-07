use std::collections::{HashMap, HashSet};
use std::env;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::context::RlcCtxt;
use crate::type_analysis::ownership::RawTypeOwner;

pub mod connect;
pub mod visitor;
pub mod backward;
pub mod ownership;

type TyMap<'tcx> = HashMap<Ty<'tcx>, String>;
type OwnerUnit = (RawTypeOwner, Vec<bool>);
type AdtOwner = HashMap<DefId, Vec<OwnerUnit>>;
type Parameters = HashSet<usize>;
pub type Unique = HashSet<DefId>;

// Type Analysis is the first step and it will perform a simple-inter-procedural analysis
// for current crate and collect types after monomorphism as well as extracting 'adt-def'.
// The struct TypeAnalysis implements mir::Visitor to simulate as the type collector.
// Note: the type in this phase is Ty::ty rather of Hir::ty.
#[derive(Clone)]
pub struct TypeAnalysis<'tcx> {
    rcx: RlcCtxt<'tcx>,
    fn_set: Unique,
    ty_map: TyMap<'tcx>,
    adt_recorder: Unique,
    adt_owner: AdtOwner,
}

impl<'tcx> TypeAnalysis<'tcx> {
    pub fn new(rcx: RlcCtxt<'tcx>) -> Self {
        Self {
            rcx,
            fn_set: HashSet::new(),
            ty_map: HashMap::new(),
            adt_recorder: HashSet::new(),
            adt_owner: HashMap::new(),
        }
    }

    pub fn rcx(&self) -> RlcCtxt<'tcx> {
        self.rcx
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.rcx().tcx()
    }

    pub fn ty_map(&self) -> &TyMap<'tcx> {
        &self.ty_map
    }

    pub fn ty_map_mut(&mut self) -> &mut TyMap<'tcx> {
        &mut self.ty_map
    }

    pub fn fn_set(&self) -> &Unique {
        &self.fn_set
    }

    pub fn fn_set_mut(&mut self) -> &mut Unique {
        &mut self.fn_set
    }

    pub fn adt_recorder(&self) -> &Unique {&self.adt_recorder}

    pub fn adt_recorder_mut(&mut self) -> &mut Unique {&mut self.adt_recorder}

    pub fn adt_owner(&self) -> &AdtOwner {&self.adt_owner}

    pub fn adt_owner_mut(&mut self) -> &mut AdtOwner {&mut self.adt_owner}

    // The main phase and the starter function of Type Collector.
    // RLC will construct an instance of struct TypeCollector and call self.start to make analysis starting.
    pub fn start(&mut self) {
        // Get the analysis result from rlc phase llvm
        self.connect();
        // Get related adt types through visiting mir local
        self.visitor();
        // Solving types by local ty and rlc llvm result
        self.backward();
    }
}

#[derive(Clone)]
struct RawGeneric<'tcx> {
    tcx: TyCtxt<'tcx>,
    record: Vec<bool>,
}

impl<'tcx> RawGeneric<'tcx> {

    pub fn new(tcx: TyCtxt<'tcx>, len: usize) -> Self {
        Self {
            tcx,
            record: vec![false ; len],
        }
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> { self.tcx }

    pub fn record(&self) -> &Vec<bool> { &self.record }

    pub fn record_mut(&mut self) -> &mut Vec<bool> { &mut self.record }
}

#[derive(Clone)]
struct RawGenericFieldSubst<'tcx> {
    tcx: TyCtxt<'tcx>,
    parameters: Parameters,
}

impl<'tcx> RawGenericFieldSubst<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            parameters: HashSet::new(),
        }
    }
    pub fn tcx(&self) -> TyCtxt<'tcx> { self.tcx }

    pub fn parameters(&self) -> &Parameters { &self.parameters }

    pub fn parameters_mut(&mut self) -> &mut Parameters { &mut self.parameters }

    pub fn contains_param(&self) -> bool { !self.parameters.is_empty() }

}


#[derive(Clone)]
struct RawGenericPropagation<'tcx, 'a> {
    tcx: TyCtxt<'tcx>,
    record: Vec<bool>,
    unique: Unique,
    source_enum: bool,
    ref_adt_owner: &'a AdtOwner,
}

impl<'tcx, 'a> RawGenericPropagation<'tcx, 'a> {
    pub fn new(tcx: TyCtxt<'tcx>, record: Vec<bool>, source_enum: bool, ref_adt_owner: &'a AdtOwner) -> Self {
        Self {
            tcx,
            record,
            unique: HashSet::new(),
            source_enum,
            ref_adt_owner,
        }
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> { self.tcx }

    pub fn record(&self) -> &Vec<bool> { &self.record }

    pub fn record_mut(&mut self) -> &mut Vec<bool> { &mut self.record }

    pub fn unique(&self) -> &Unique { &self.unique }

    pub fn unique_mut(&mut self) -> &mut Unique { &mut self.unique }

    pub fn source_enum(&mut self) -> bool { self.source_enum }

    pub fn owner(&self) -> &'a AdtOwner { self.ref_adt_owner }

}

#[derive(Clone)]
struct OwnerPropagation<'tcx, 'a> {
    tcx: TyCtxt<'tcx>,
    ownership: RawTypeOwner,
    unique: Unique,
    ref_adt_owner: &'a AdtOwner,
}

impl<'tcx, 'a> OwnerPropagation<'tcx, 'a> {
    pub fn new(tcx: TyCtxt<'tcx>, ownership: RawTypeOwner, ref_adt_owner: &'a AdtOwner) -> Self {
        Self {
            tcx,
            ownership,
            unique: HashSet::new(),
            ref_adt_owner,
        }
    }

    pub fn tcx(&self) -> TyCtxt<'tcx> { self.tcx }

    pub fn ownership(&self) -> RawTypeOwner { self.ownership }

    pub fn unique(&self) -> &Unique { &self.unique }

    pub fn unique_mut(&mut self) -> &mut Unique { &mut self.unique }

    pub fn owner(&self) -> &'a AdtOwner { self.ref_adt_owner }

}

#[derive(Debug, Copy, Clone, Hash)]
pub enum AdtOwnerDisplay {
    Verbose,
    Disabled,
}

pub fn is_display_verbose() -> bool {
    match env::var_os("ADT_DISPLAY") {
        Some(verbose)  => true,
        _ => false,
    }
}