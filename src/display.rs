use std::env;

use rustc_middle::mir::terminator::{Terminator, TerminatorKind};
use rustc_middle::mir::{Operand, Rvalue, Statement, StatementKind, BasicBlock, BasicBlockData, Body};
use rustc_middle::ty;
use rustc_index::vec::IndexVec;


// In this crate we will costume the display information for Compiler-Metadata by implementing
// rlc::Display for target data structure.
// If you want print these meg to terminal that makes debug easily, please add -v or -vv to rlc
// that makes entire rlc verbose.
pub trait Display {
    fn display(&self) -> String;
}

const NEXT_LINE:&str = "\n";
const PADDING:&str = "    ";

impl<'tcx> Display for Terminator<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{}{:?}", self.kind.display(), PADDING, self.kind);
        s
    }
}

impl<'tcx> Display for TerminatorKind<'tcx>{
    fn display(&self) -> String {
        let mut s = String::new();
        if env::var_os("RLC_VERBOSE").is_some() {
            s += PADDING;
            match self {
                TerminatorKind::Goto { target:_ } =>
                    s += "$$$$$ Goto",
                TerminatorKind::SwitchInt { discr:_, switch_ty:_, targets:_ } =>
                    s += "$$$$$ SwitchInt",
                TerminatorKind::Resume =>
                    s += "$$$$$ Resume",
                TerminatorKind::Abort =>
                    s += "$$$$$ Abort",
                TerminatorKind::Return =>
                    s += "$$$$$ Return",
                TerminatorKind::Unreachable =>
                    s += "$$$$$ Unreachable",
                TerminatorKind::Drop { place:_, target:_, unwind:_ } =>
                    s += "$$$$$ Drop",
                TerminatorKind::DropAndReplace { place:_, value:_, target:_, unwind:_ } =>
                    s += "$$$$$ DropAndReplace",
                TerminatorKind::Assert { cond:_, expected:_, msg:_, target:_, cleanup:_ } =>
                    s += "$$$$$ Assert",
                TerminatorKind::Yield { value:_, resume:_, resume_arg:_, drop:_ } =>
                    s += "$$$$$ Yield",
                TerminatorKind::GeneratorDrop =>
                    s += "$$$$$ GeneratorDrop",
                TerminatorKind::FalseEdge { real_target:_, imaginary_target:_ } =>
                    s += "$$$$$ FalseEdge",
                TerminatorKind::FalseUnwind { real_target:_, unwind:_ } =>
                    s += "$$$$$ FalseUnwind",
                TerminatorKind::InlineAsm { template:_, operands:_, options:_, line_spans:_, destination:_ } =>
                    s += "$$$$$ InlineAsm",
                TerminatorKind::Call { ref func, args:_, destination:_, cleanup:_, from_hir_call:_, fn_span:_ } => {
                    match func {
                        Operand::Constant(ref constant) => {
                                match constant.literal.ty().kind() {
                                    ty::FnDef(ref id, _) =>
                                        s += &format!("$$$$$ Call FnDef ID is {}", id.index.as_usize()).as_str(),
                                    _ => (),
                                }
                        },
                        _ => (),
                    }
                }
            };
            s += NEXT_LINE;
        } else {
            ()
        };
        s
    }
}

impl<'tcx> Display for Statement<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{}{:?}", self.kind.display(), PADDING, self.kind);
        s
    }
}

impl<'tcx> Display for StatementKind<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if env::var_os("RLC_VERBOSE").is_some() {
            s += PADDING;
            match self {
                StatementKind::Assign(ref assign) => {
                    s += "@@@@@ Assign";
                    s += &format!("    {:?} {}", assign.0, assign.1.display());
                }
                StatementKind::FakeRead(_) =>
                    s += "@@@@@ FakeRead",
                StatementKind::SetDiscriminant { place: _, variant_index:_ } =>
                    s += "@@@@@ SetDiscriminant",
                StatementKind::StorageLive(_) =>
                    s += "@@@@@ StorageLive",
                StatementKind::StorageDead(_) =>
                    s += "@@@@@ StorageDead",
                StatementKind::LlvmInlineAsm(_) =>
                    s += "@@@@@ LlvmInlineAsm",
                StatementKind::Retag(_, _) =>
                    s += "@@@@@ Retag",
                StatementKind::AscribeUserType(_, _) =>
                    s += "@@@@@ AscribeUserType",
                StatementKind::Coverage(_) =>
                    s += "@@@@@ Coverage",
                StatementKind::CopyNonOverlapping(_) =>
                    s += "@@@@@ CopyNonOverlapping",
                StatementKind::Nop =>
                    s += "@@@@@ Nop",
            }
            s += NEXT_LINE;
        } else {
            ()
        }
        s
    }
}

impl<'tcx> Display for Rvalue<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if env::var_os("RLC_VERBOSE").is_some() {
            match self {
                Rvalue::Use(_) =>
                    s += "Rvalue Use",
                Rvalue::Repeat(_, _) =>
                    s += "Rvalue Repeat",
                Rvalue::Ref(_, _, _) =>
                    s += "Rvalue Ref",
                Rvalue::ThreadLocalRef(_) =>
                    s += "Rvalue ThreadLocalRef",
                Rvalue::AddressOf(_, _) =>
                    s += "Rvalue AddressOf",
                Rvalue::Len(_) =>
                    s += "Rvalue Len",
                Rvalue::Cast(_, _, _) =>
                    s += "Rvalue Cast",
                Rvalue::BinaryOp(_, _) =>
                    s += "Rvalue BinaryOp",
                Rvalue::CheckedBinaryOp(_, _) =>
                    s += "Rvalue CheckedBinaryOp",
                Rvalue::NullaryOp(_, _) =>
                    s += "Rvalue NullaryOp",
                Rvalue::UnaryOp(_, _) =>
                    s += "Rvalue UnaryOp",
                Rvalue::Discriminant(_) =>
                    s += "Rvalue Discriminant",
                Rvalue::Aggregate(_, _) =>
                    s += "Rvalue Aggregate",
                Rvalue::ShallowInitBox(_, _) =>
                    s+= "Rvalue ShallowInitBox",
            }
        } else {
            ()
        }
        s
    }
}

type BasicBlocks<'tcx> = IndexVec<BasicBlock, BasicBlockData<'tcx>>;

impl<'tcx> Display for BasicBlocks<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        let mut count = 0;
        for bb in self.iter() {
            s += &format!("bb {} {{{}{}}}{}", count, NEXT_LINE, bb.display(), NEXT_LINE);
            count = count + 1;
        }
        s
    }
}

impl<'tcx> Display for BasicBlockData<'tcx>  {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("UNWIND BLOCK: {}{}", self.is_cleanup, NEXT_LINE);
        for stmt in self.statements.iter() {
            s += &format!("{}{}", stmt.display(), NEXT_LINE);
        }
        s += &format!("{}{}", self.terminator.clone().unwrap().display(), NEXT_LINE);
        s
    }
}

impl<'tcx> Display for Body<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &self.basic_blocks().display();
        s
    }
}