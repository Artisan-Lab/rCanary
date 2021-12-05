use rustc_middle::ty::{self, Ty, TyCtxt, TyKind, TypeVisitor, TypeFoldable};
use rustc_middle::ty::subst::GenericArgKind;
use rustc_middle::mir::visit::{Visitor, TyContext};
use rustc_middle::mir::{Body, BasicBlock, BasicBlockData, Local, LocalDecl, Operand};
use rustc_middle::mir::terminator::TerminatorKind;
use rustc_span::def_id::DefId;

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::thread::sleep;

use super::{TypeAnalysis, AdtOwner, FoundParam, RawGeneric, RawTypeOwner};
use crate::display::{self, Display};
use crate::type_analysis::{RawGenericFieldSubst, RawGenericPropagation};

pub(crate) fn mir_body<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &'tcx Body<'tcx> {
    let id = ty::WithOptConstParam::unknown(def_id);
    let def = ty::InstanceDef::Item(id);
    tcx.instance_mir(def)
}

// This function is aiming at resolving problems due to 'TyContext' not implementing 'Clone' trait,
// thus we call function 'copy_ty_context' to simulate 'self.clone()'.
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

        let dids:Vec<DefId> = self.adt_recorder.iter().map(|did| *did).collect();

        for did in &dids {
            self.extract_raw_generic(*did);
        }

        for did in &dids {
            self.extract_raw_generic_prop(*did);
        }

        for did in &dids {
            self.extract_phantom_unit(*did);
        }

        // for did in &dids {
        //     self.extract_owner_prop(*did);
        // }

        for elem in &self.adt_owner {
            println!("{:?} {:?}", self.tcx().type_of(*elem.0), elem.1);
            // if elem.1.0 != RawTypeOwner::Unowned {
            //     println!("{:?} {:?}", self.tcx().type_of(*elem.0), elem.1);
            // }
        }

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
    fn extract_raw_generic(&mut self, did: DefId) {

        // Get the definition and subset reference from adt did
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        if adt_def.is_struct() {

            let mut raw_generic = RawGeneric::new(self.tcx(), substs.len());

            for field in adt_def.all_fields() {
                let field_ty = field.ty(self.tcx(), substs);
                field_ty.visit_with(&mut raw_generic);
            }

            let mut res:(RawTypeOwner, Vec<bool>) = (RawTypeOwner::Unowned, raw_generic.record.clone());

            self.adt_owner_mut().insert(did, res);
        }
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
    //
    // struct Y1<A, B> {
    //     a: A,
    //     b: (i32, (f64, B)),
    //     c: [[(S) ; 1] ; 2],
    //     d: Vec<T>,
    // }
    //
    // struct Example<A, B, T, S> {
    //     a: A,
    //     b: (i32, (f64, B)),
    //     c: [[(S) ; 1] ; 2],
    //     d: Vec<T>,
    // }
    //
    // the final result for <A, B, T, S> is <true, true, false, true>.
    fn extract_raw_generic_prop(&mut self, did: DefId) {

        // Get the definition and subset reference from adt did
        let ty = self.tcx().type_of(did);
        let (adt_def, substs) = match ty.kind() {
            TyKind::Adt(adt_def, substs) => (adt_def, substs),
            _ => unreachable!(),
        };

        if adt_def.is_struct() {

            println!("This is for {} ", ty);

            let mut res = self.adt_owner_mut().get_mut(&did).unwrap().clone();
            let record = res.1.clone();

            let mut raw_generic_prop = RawGenericPropagation::new(
                self.tcx(),
                record,
                self.adt_owner()
            );

            for field in adt_def.all_fields() {
                let field_ty = field.ty(self.tcx(), substs);
                println!("     Field: {}", field_ty);
                field_ty.visit_with(&mut raw_generic_prop);
            }

            res.1 = raw_generic_prop.record_mut().clone();
            self.adt_owner_mut().insert(did, res);
        }

    }

    // Extract all types that include PhantomData<T> which T must be a raw Param
    // Consider these types as a unit to guide the traversal over adt types
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
            let mut res = self.adt_owner_mut().get_mut(&did).unwrap().clone();
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
                                    GenericArgKind::Lifetime(..) => { return; },
                                    GenericArgKind::Type(ty) => {
                                        match ty.kind() {
                                            TyKind::Param(_) => break,
                                            _ => { return; }
                                        }
                                    },
                                    GenericArgKind::Const(..) => { return; },
                                }
                            }
                            res.0 = RawTypeOwner::Owned;
                            self.adt_owner_mut().insert(did, res);
                            break;
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    // fn extract_did_from_unit(&mut self, did: DefId) {
    //
    //     if self.adt_result().contains_key(&did) {
    //         return;
    //     }
    //
    //     let ty = self.tcx().type_of(did);
    //     let (adt_def, substs) = match ty.kind() {
    //         TyKind::Adt(adt_def, substs) => (adt_def, substs),
    //         _ => unreachable!(),
    //     };
    //
    //     if adt_def.is_struct() {
    //
    //         let mut res:(RawTypeOwner, Vec<bool>) = (RawTypeOwner::Unowned, vec![false ; substs.len()]);
    //         for field in adt_def.all_fields() {
    //
    //             let field_ty = field.ty(self.tcx(), substs);
    //             match field_ty.kind() {
    //                 // For one field which is param like T A K V, calculate times that these params appear in this struct
    //                 TyKind::Param(param_ty) => {
    //                     res.1[param_ty.index as usize] = res.1[param_ty.index as usize] + 1;
    //                 },
    //                 // For one field which is a struct type, perform get the analysis result from map
    //                 TyKind::Adt(field_adt_def, field_substs) => {
    //
    //                     // if !self.adt_result.contains_key(&field_adt_def.did) {
    //                     //     self.extract_did_from_unit(field_adt_def.did);
    //                     // }
    //
    //                     // let field_res = self.adt_result().get(&field_adt_def.did).unwrap();
    //
    //                     // res.0 = res.0 + field_res.0;
    //                     // if !field_res.1.iter().any( |num| *num != 0 ) {
    //                     //     continue;
    //                     // }
    //
    //                     if self.adt_result.contains_key(&field_adt_def.did) {
    //                         let field_res = self.adt_result().get(&field_adt_def.did).unwrap();
    //                         res.0 = res.0 + field_res.0;
    //                         if !field_res.1.iter().any( |num| *num != 0 ) {
    //                             continue;
    //                         }
    //
    //                         for (index, field_generic_arg_cnt) in field_res.1.iter().enumerate() {
    //                             if *field_generic_arg_cnt == 0 { continue; }
    //                             let field_generic_arg = field_substs[index];
    //                             let field_param_ty = match field_generic_arg.unpack() {
    //                                 GenericArgKind::Lifetime(..) => { continue; },
    //                                 GenericArgKind::Type(ty) => {
    //                                     match ty.kind() {
    //                                         TyKind::Param(param_ty) => param_ty,
    //                                         _ => { return; }
    //                                     }
    //                                 },
    //                                 GenericArgKind::Const(..) => { continue; },
    //                             };
    //                             res.1[field_param_ty.index as usize] = res.1[field_param_ty.index as usize] + field_res.1[index];
    //                         }
    //                     }
    //                 },
    //                 // TyKind::Array(ty, _const) => {
    //                 //
    //                 // },
    //                 // TyKind::Tuple(..) => {
    //                 //
    //                 // },
    //                 _ => continue,
    //             }
    //
    //         }
    //         self.adt_result_mut().insert(adt_def.did, res);
    //         return;
    //     }
    //
    //
    //     if adt_def.is_enum() {
    //
    //         return;
    //     }
    //
    // }
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
            TyKind::Array(ty, _const) => {
                self.visit_ty(ty, ty_context);
            },
            TyKind::Slice(ty) => {
                self.visit_ty(ty, ty_context);
            },
            TyKind::RawPtr(typeandmut) => {
                let ty = typeandmut.ty;
                self.visit_ty(ty, ty_context);
            },
            TyKind::Ref(_region, ty, _mutability) => {
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
        match term.kind {
            TerminatorKind::Call { ref func, args:_, destination:_, cleanup:_, from_hir_call:_, fn_span:_ } => {
                match func {
                    Operand::Constant(ref constant) => {
                        match constant.literal.ty().kind() {
                            ty::FnDef(def_id, _) => {
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

    type BreakTy = FoundParam;

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
        match ty.kind() {
            TyKind::Array(..) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple(..) => {
                ty.super_visit_with(self)
            },
            TyKind::Param(param_ty) => {
                self.record_mut()[param_ty.index as usize] = true;
                ControlFlow::Break(FoundParam)
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }
}

impl<'tcx> TypeVisitor<'tcx> for RawGenericFieldSubst<'tcx> {
    type BreakTy = ();

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
        match ty.kind() {
            TyKind::Array(..) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple(..) => {
                ty.super_visit_with(self)
            },
            TyKind::Adt(..) => {
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

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {

        match ty.kind() {
            TyKind::Adt(adtdef, substs) => {
                if substs.len() == 0 { return ControlFlow::CONTINUE; }

                let mut map_raw_generic_field_subst = HashMap::new();

                for (index, subst) in substs.iter().enumerate() {
                    match subst.unpack() {
                        GenericArgKind::Lifetime(_) => continue,
                        GenericArgKind::Const(_) => continue,
                        GenericArgKind::Type(g_ty) => {
                            let mut raw_generic_field_subst = RawGenericFieldSubst::new(self.tcx());
                            g_ty.visit_with(&mut raw_generic_field_subst);
                            if !raw_generic_field_subst.contains_param() { continue; }
                            map_raw_generic_field_subst.insert(index as usize, raw_generic_field_subst);
                        }
                    }
                }
                if map_raw_generic_field_subst.is_empty() { return ControlFlow::CONTINUE; }

                if !self.unique_mut().insert(adtdef.did) { return ControlFlow::CONTINUE; }

                let get_ans = self.owner().get(&adtdef.did);

                // Fixme: need support for enum
                if get_ans.is_none() {
                    self.unique.remove(&adtdef.did);
                    return ControlFlow::CONTINUE;
                }

                let get_ans = get_ans.unwrap();
                for (index, flag) in  get_ans.1.iter().enumerate() {
                    if *flag && map_raw_generic_field_subst.contains_key(&index) {
                        for elem in map_raw_generic_field_subst.get(&index).unwrap().parameters() {
                            self.record[*elem] = true;
                            println!("          param: {} ; ans_index: {}", elem, index);
                        }
                    }
                }

                for field in adtdef.all_fields() {
                    let field_ty = field.ty(self.tcx(), substs);
                    match field_ty.kind() {
                        TyKind::Adt(adtdef, substs) => {
                            println!("ty:{} ans:{:?} field:{} sub:{:?}",ty, get_ans, field_ty, substs);
                            if substs.len() > 0 {
                                for elem in substs.types() {
                                    match elem.kind() {
                                        TyKind::Param(x) => println!("{:?}", x),
                                        _ => {},
                                    };
                                    break;
                                }
                            }

                        },
                        _ => {}
                    }
                    field_ty.visit_with(self);
                }

                self.unique.remove(&adtdef.did);

                ty.super_visit_with(self)
            }
            TyKind::Array(..) => {
                ty.super_visit_with(self)
            },
            TyKind::Tuple(..) => {
                ty.super_visit_with(self)
            },
            _ => {
                ControlFlow::CONTINUE
            },
        }
    }

}