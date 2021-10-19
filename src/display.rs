use core::fmt;
use std::env;
use std::fmt::Formatter;

use rustc_middle::mir::terminator::{Terminator, TerminatorKind};
use rustc_middle::mir::Operand;
use rustc_middle::ty;


// In this crate we will costume the display information for Compiler-Metadata by implementing
// rlc::Display for target data structure.
pub trait Display {
    fn display(&self) -> String;
}

const NEXT_LINE:&str = "\n";

impl<'tcx> Display for Terminator<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{}{:?}", self.kind.display(), NEXT_LINE, self.kind);
        s
    }
}

impl<'tcx> Display for TerminatorKind<'tcx>{
    fn display(&self) -> String {
        let mut s = String::new();

        if env::var_os("RLC_VERBOSE").is_some() {
            match self {
                TerminatorKind::Goto { target:_ } =>
                    s += "$TERM Goto",
                TerminatorKind::SwitchInt { discr:_, switch_ty:_, targets:_ } =>
                    s += "$TERM SwitchInt",
                TerminatorKind::Resume =>
                    s += "$TERM Resume",
                TerminatorKind::Abort =>
                    s += "$TERM Abort",
                TerminatorKind::Return =>
                    s += "$TERM Return",
                TerminatorKind::Unreachable =>
                    s += "$TERM Unreachable",
                TerminatorKind::Drop { place:_, target:_, unwind:_ } =>
                    s += "$TERM Drop",
                TerminatorKind::DropAndReplace { place:_, value:_, target:_, unwind:_ } =>
                    s += "$TERM DropAndReplace",
                TerminatorKind::Assert { cond:_, expected:_, msg:_, target:_, cleanup:_ } =>
                    s += "$TERM Assert",
                TerminatorKind::Yield { value:_, resume:_, resume_arg:_, drop:_ } =>
                    s += "$TERM Yield",
                TerminatorKind::GeneratorDrop =>
                    s += "$TERM GeneratorDrop",
                TerminatorKind::FalseEdge { real_target:_, imaginary_target:_ } =>
                    s += "$TERM FalseEdge",
                TerminatorKind::FalseUnwind { real_target:_, unwind:_ } =>
                    s += "$TERM FalseUnwind",
                TerminatorKind::InlineAsm { template:_, operands:_, options:_, line_spans:_, destination:_ } =>
                    s += "$TERM InlineAsm",
                TerminatorKind::Call { ref func, args:_, destination:_, cleanup:_, from_hir_call:_, fn_span:_ } => {
                    match func {
                        Operand::Constant(ref constant) => {
                                match constant.literal.ty().kind() {
                                    ty::FnDef(ref id, _) =>
                                        s += &format!("$TERM Call FnDef ID is {}", id.index.as_usize()).as_str(),
                                    _ => (),
                                }
                        },
                        _ => (),
                    }
                }
            };
        } else {
            ()
        };
        s
    }
}

