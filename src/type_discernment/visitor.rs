use rustc_middle::ty::{self, Ty, TyCtxt, TyKind};
use rustc_middle::mir::visit::{Visitor, TyContext};
use rustc_middle::mir::{Body, BasicBlock, BasicBlockData, Local, LocalDecl, Operand};
use rustc_middle::mir::terminator::TerminatorKind;
use rustc_span::def_id::DefId;

use super::TypeDiscernment;
use crate::display::{self, Display};

impl<'tcx> TypeDiscernment<'tcx> {
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
            if self.func_set_mut().insert(def_id) {
                self.visit_body(body);
            } else {
                continue;
            }
        }

        println!("{:?}", self.type_map.ty_to_string);


    }
}

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

        // This function is to resolve the problem due to TyContext does not impl Clone
        fn copy_ty_context(tc: &TyContext) -> TyContext {
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

        match ty.kind() {
            TyKind::Adt(_adtdef, substs) => {

                let name = format!("{:?}", ty);
                if self.type_map().get_t2s().get(ty).is_some() {
                    return;
                }

                self.type_map_mut().insert_ty(ty);

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
                                if self.tcx().is_mir_available(*def_id) {
                                    let body = mir_body(self.tcx(), *def_id);
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


fn mir_body<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &'tcx Body<'tcx> {
    let id = ty::WithOptConstParam::unknown(def_id);
    let def = ty::InstanceDef::Item(id);
    tcx.instance_mir(def)
}

