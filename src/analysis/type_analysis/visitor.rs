use rustc_middle::ty::{self, Ty, TyCtxt, TyKind, TypeVisitor, TypeFoldable};
use rustc_middle::ty::subst::GenericArgKind;
use rustc_middle::mir::visit::{Visitor, TyContext};
use rustc_middle::mir::{Body, BasicBlock, BasicBlockData, Local, LocalDecl, Operand};
use rustc_middle::mir::terminator::TerminatorKind;
use rustc_span::def_id::DefId;

use std::collections::HashMap;
use std::ops::ControlFlow;

use crate::display::{self, Display};
use crate::type_analysis::{TypeAnalysis, OwnerPropagation, RawGeneric, RawGenericFieldSubst, RawGenericPropagation, RawTypeOwner};
use crate::type_analysis::ownership::RawTypeOwner::Owned;

pub(crate) fn mir_body<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &'tcx Body<'tcx> {
    let id = ty::WithOptConstParam::unknown(def_id);
    let def = ty::InstanceDef::Item(id);
    tcx.instance_mir(def)
}

// This function is aiming at resolving problems due to 'TyContext' not implementing 'Clone' trait,
// thus we call function 'copy_ty_context' to simulate 'self.clone()'.
#[inline(always)]
pub(crate) fn copy_ty_context(tc: &TyContext) -> TyContext {
    match tc {
        TyContext::LocalDecl { local, source_info } => {
            TyContext::LocalDecl {
                local: local.clone(),
                source_info: source_info.clone(),
            }
        },
        _ => unreachable!(),
    }
}

impl<'tcx> TypeAnalysis<'tcx> {
    // The 'visitor' method is our main pass of the constructor part in type analysis,
    // it will perform several important procedural to determine whether an adt definition (adt-def)
    // will occupy at least one heap allocation, reflecting holding heap-ownership in RLC system.
    //
    // From the top-down method of our approach, this 'visitor' is the set of several sub-phases
    // which means it contains multiple sub-visitors to make whole method 'self.visitor()' work.
    //
    // For example, given an adtef (like Vec<T>), the result of 'visitor' contains two parts:
    //
    //     pt1 Enum:  {Owned / UnOwned} indicates whether it will directly have a heap data
    //     pt2 Array: [bool;N] indicates whether each generic parameter will have a raw param
    //
    // Those 2 parts can accelerate heap-ownership inference in the data-flow analysis.
    pub fn visitor(&mut self) {

        #[inline(always)]
        fn start_channel<M>(mut method: M, v_did: &Vec<DefId>)
            where M: FnMut(DefId) -> (),
        {
            for did in v_did {
                method(*did);
            }
        }

        // Get the Global TyCtxt from rustc
        // Grasp all mir Keys defined in current crate
        let tcx = self.tcx();
        let mir_keys = tcx.mir_keys(());

        for each_mir in mir_keys {
            // Get the defid of current crate and get mir Body through this id
            let def_id = each_mir.to_def_id();
            let body = mir_body(tcx, def_id);

            // Insert the defid to hashset if is not existed and visit the body
            if self.fn_set_mut().insert(def_id) {
                self.visit_body(body);
            } else {
                continue;
            }
        }

        let dids: Vec<DefId> = self.adt_recorder.iter().map(|did| *did).collect();

        start_channel(|did| self.extract_raw_generic(did), &dids);
        start_channel(|did| self.extract_raw_generic_prop(did), &dids);
        start_channel(|did| self.extract_phantom_unit(did), &dids);
        start_channel(|did| self.extract_owner_prop(did), &dids);

        // for elem in &self.adt_owner {
        //     println!("{:?} {:?}", self.tcx().type_of(*elem.0), elem.1);
        //     if elem.1.0 != RawTypeOwner::Unowned {
        //         println!("{:?} {:?}", self.tcx().type_of(*elem.0), elem.1);
        //     }
        // }
    }

    // Extract params in adt types, the 'param' means one generic parameter acting like 'T', 'A', etc...
    // In the sub-visitor RawGeneric, it will visit the given type recursively, and extract all params.
    //
    // Note that rlc is only interested in 'raw' params ('T' not like '*mut T').
    // It lies in 'one-entire field' | recursive in tuple | recursive in array | mixed before
    //
    // Given a struct Example<A, B, T, S>:
    //
    // struct Example<A, B, T, S> {
    //     a: A,
    //     b: (i32, (f64, B)),
    //     c: [[(S) ; 1] ; 2],
    //     d: Vec<T>,
    // }
    //
    // the final result for <A, B, T, S> is <true, true, false, true>.
    #[inline(always)]
    fn extract_raw_generic(&mut self, did: DefId) {

        // Get the definition and subset reference from adt did
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        let mut v_res = Vec::new();

        for variant in adt_def.variants.iter() {
            let mut raw_generic = RawGeneric::new(self.tcx(), substs.len());

            for field in &variant.fields {
                let field_ty = field.ty(self.tcx(), substs);
                field_ty.visit_with(&mut raw_generic);
            }
            v_res.push((RawTypeOwner::Unowned, raw_generic.record_mut().clone()));
        }

        self.adt_owner_mut().insert(did, v_res);

    }

    // Extract all params in the adt types like param 'T' and then propagate from the bottom to top.
    // This procedural is the successor of `extract_raw_generic`, and the main idea of RawGenericPropagation
    // is to propagate params from bottom adt to the top as well as updating TypeAnalysis Context.
    //
    // Note that it will thorough consider mono-morphization existed in adt-def.
    // That means the type 'Vec<T>', 'Vec<Vec<T>>' and 'Vec<i32>' are totally different!!!!
    //
    // Given a struct Example<A, B, T, S>:
    //
    // struct X<A> {
    //     a: A,
    // }
    // the final result for <A> is <true>.
    //
    // struct Y1<B> {
    //     a: (i32, (f64, B)),
    //     b: X<i32>,
    // }
    // the final result for <B> is <true>.
    //
    // struct Example<A, B, T, S> {
    //     a: X<A>,
    //     b: (i32, (f64, B)),
    //     c: [[(S) ; 1] ; 2],
    //     d: Vec<T>,
    // }
    //
    // the final result for <A, B, T, S> is <true, true, false, true>.
    #[inline(always)]
    fn extract_raw_generic_prop(&mut self, did: DefId) {

        // Get the definition and subset reference from adt did
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        let source_enum = adt_def.is_enum();

        let mut v_res = self.adt_owner_mut().get_mut(&did).unwrap().clone();

        for (variant_index, variant) in adt_def.variants.iter().enumerate() {
            let res = v_res[variant_index as usize].clone();

            let mut raw_generic_prop = RawGenericPropagation::new(
                self.tcx(),
                res.1.clone(),
                source_enum,
                self.adt_owner()
            );

            for field in &variant.fields {
                let field_ty = field.ty(self.tcx(), substs);
                field_ty.visit_with(&mut raw_generic_prop);
            }
            v_res[variant_index as usize] = (RawTypeOwner::Unowned, raw_generic_prop.record_mut().clone());
        }

        self.adt_owner_mut().insert(did, v_res);

    }

    // Extract all types that include PhantomData<T> which T must be a raw Param
    // Consider these types as a unit to guide the traversal over adt types
    #[inline(always)]
    fn extract_phantom_unit(&mut self, did: DefId) {

        // Get ty from defid and the ty is made up with generic type
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        // As for one heap-allocation unit, only struct will contains the information that we want
        // Example:
        // struct Foo<T> {
        //     NonNull<T>,      // this indicates a pointer
        //     PhantomData<T>,  // this indicates a ownership
        // }
        if adt_def.is_struct() {
            let mut res = self.adt_owner_mut().get_mut(&did).unwrap()[0].clone();
            // Extract all fields in one given struct
            for field in adt_def.all_fields() {
                let field_ty = field.ty(self.tcx(), substs);
                match field_ty.kind() {
                    // Filter the field which is also a struct due to PhantomData<T> is struct
                    TyKind::Adt(field_adt_def, field_substs) => {
                        if field_adt_def.is_phantom_data() {
                            // Extract all generic args in the type
                            for generic_arg in *field_substs {
                                match generic_arg.unpack() {
                                    GenericArgKind::Type( g_ty ) => {
                                        let mut raw_generic_field_subst = RawGenericFieldSubst::new(self.tcx());
                                        g_ty.visit_with(&mut raw_generic_field_subst);
                                        if raw_generic_field_subst.contains_param() {
                                            res.0 = RawTypeOwner::Owned;
                                            self.adt_owner_mut().insert(did, vec![res.clone()]);
                                            return;
                                        }
                                    },
                                    GenericArgKind::Lifetime( .. ) => { return; },
                                    GenericArgKind::Const( .. ) => { return; },
                                }
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    #[inline(always)]
    fn extract_owner_prop(&mut self, did: DefId) {

        // Get the definition and subset reference from adt did
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        let mut v_res = self.adt_owner_mut().get_mut(&did).unwrap().clone();

        for (variant_index, variant) in adt_def.variants.iter().enumerate() {
            let res = v_res[variant_index as usize].clone();

            let mut owner_prop = OwnerPropagation::new(
                self.tcx(),
                res.0,
                self.adt_owner()
            );

            for field in &variant.fields {
                let field_ty = field.ty(self.tcx(), substs);
                field_ty.visit_with(&mut owner_prop);
            }
            v_res[variant_index as usize].0 = owner_prop.ownership();
        }

        self.adt_owner_mut().insert(did, v_res);
    }
}


impl<'tcx> Visitor<'tcx> for TypeAnalysis<'tcx> {
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

        match ty.kind() {
            TyKind::Adt(adtdef, substs) => {

                if self.ty_map().get(ty).is_some() {
                    return;
                }
                self.ty_map_mut().insert(ty, format!("{:?}", ty));
                self.adt_recorder_mut().insert(adtdef.did);

                for field in adtdef.all_fields() {
                    self.visit_ty(field.ty(self.tcx(), substs) ,copy_ty_context(&ty_context))
                }

                for ty in substs.types() {
                    self.visit_ty(ty, copy_ty_context(&ty_context));
                }
            },
            TyKind::Array(ty, ..) => {
                self.visit_ty(ty, ty_context);
            },
            TyKind::Slice(ty) => {
                self.visit_ty(ty, ty_context);
            },
            TyKind::RawPtr(typeandmut) => {
                let ty = typeandmut.ty;
                self.visit_ty(ty, ty_context);
            },
            TyKind::Ref(_, ty, ..) => {
                self.visit_ty(ty, ty_context);
            },
            TyKind::Tuple(substs) => {
                for tuple_ty in ty.tuple_fields() {
                    self.visit_ty(tuple_ty, copy_ty_context(&ty_context));
                }
                for ty in substs.types() {
                    self.visit_ty(ty, copy_ty_context(&ty_context));
                }
            },
            _ => return,
        }
    }

    fn visit_basic_block_data(
        &mut self,
        _block: BasicBlock,
        data: &BasicBlockData<'tcx>
    ) {
        let term = data.terminator();
        match &term.kind {
            TerminatorKind::Call { func, .. } => {
                match func {
                    Operand::Constant(constant) => {
                        match constant.literal.ty().kind() {
                            ty::FnDef(def_id, ..) => {
                                if self.tcx().is_mir_available(*def_id) && self.fn_set_mut().insert(*def_id) {
                                    let body = mir_body(self.tcx(), *def_id); //
                                    self.visit_body(body);
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

impl<'tcx> TypeVisitor<'tcx> for RawGeneric<'tcx>  {

    type BreakTy = ();

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
        match ty.kind() {
            TyKind::Array( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Param(param_ty) => {
                self.record_mut()[param_ty.index as usize] = true;
                ControlFlow::CONTINUE
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }
}

impl<'tcx> TypeVisitor<'tcx> for RawGenericFieldSubst<'tcx> {
    type BreakTy = ();

    #[inline(always)]
    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    #[inline(always)]
    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
        match ty.kind() {
            TyKind::Array( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Adt( .. ) => {
                ty.super_visit_with(self)
            }
            TyKind::Param(param_ty) => {
                self.parameters_mut().insert(param_ty.index as usize);
                ControlFlow::CONTINUE
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }

}

impl<'tcx, 'a> TypeVisitor<'tcx> for RawGenericPropagation<'tcx, 'a>  {
    type BreakTy = ();

    #[inline(always)]
    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    #[inline(always)]
    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {

        match ty.kind() {
            TyKind::Adt(adtdef, substs) => {
                if substs.len() == 0 { return ControlFlow::Break(()); }

                if !self.source_enum() && adtdef.is_enum() { return ControlFlow::Break(()); }

                if !self.unique_mut().insert(adtdef.did) { return ControlFlow::CONTINUE; }

                let mut map_raw_generic_field_subst = HashMap::new();
                for (index, subst) in substs.iter().enumerate() {
                    match subst.unpack() {
                        GenericArgKind::Lifetime( .. ) => continue,
                        GenericArgKind::Const( .. ) => continue,
                        GenericArgKind::Type(g_ty) => {
                            let mut raw_generic_field_subst = RawGenericFieldSubst::new(self.tcx());
                            g_ty.visit_with(&mut raw_generic_field_subst);
                            if !raw_generic_field_subst.contains_param() { continue; }
                            map_raw_generic_field_subst.insert(index as usize, raw_generic_field_subst);
                        }
                    }
                }
                if map_raw_generic_field_subst.is_empty() { return ControlFlow::Break(()); }

                let get_ans = self.owner().get(&adtdef.did).unwrap();
                if get_ans.len() == 0 { return ControlFlow::Break(()); }
                let get_ans = get_ans[0].clone();

                for (index, flag) in  get_ans.1.iter().enumerate() {
                    if *flag && map_raw_generic_field_subst.contains_key(&index) {
                        for elem in map_raw_generic_field_subst.get(&index).unwrap().parameters() {
                            self.record[*elem] = true;
                        }
                    }
                }

                for field in adtdef.all_fields() {
                    let field_ty = field.ty(self.tcx(), substs);
                    field_ty.visit_with(self);
                }

                self.unique_mut().remove(&adtdef.did);

                ty.super_visit_with(self)
            }
            TyKind::Array( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple( .. ) => {
                ty.super_visit_with(self)
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }

}

impl<'tcx, 'a> TypeVisitor<'tcx> for OwnerPropagation<'tcx, 'a> {
    type BreakTy = ();

    #[inline(always)]
    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    #[inline(always)]
    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {

        match ty.kind() {
            TyKind::Adt(adtdef, substs) => {
                if !self.unique_mut().insert(adtdef.did) { return ControlFlow::CONTINUE; }

                if adtdef.is_enum() { return ControlFlow::Break(()); }

                let get_ans = self.owner().get(&adtdef.did).unwrap();
                if get_ans.len() == 0 { return ControlFlow::Break(()); }
                let get_ans = get_ans[0].clone();

                match get_ans.0 {
                    RawTypeOwner::Owned => { self.ownership = Owned; }
                    _ => (),
                };

                for field in adtdef.all_fields() {
                    let field_ty = field.ty(self.tcx(), substs);
                    field_ty.visit_with(self);
                }

                self.unique_mut().remove(&adtdef.did);

                ty.super_visit_with(self)
            },
            TyKind::Array( .. ) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple( .. ) => {
                ty.super_visit_with(self)
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }
}