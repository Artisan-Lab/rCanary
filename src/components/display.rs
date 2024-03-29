use rustc_middle::mir::{Operand, Rvalue, Statement, StatementKind, TerminatorKind, BasicBlock,
                        BasicBlockData, Body, LocalDecl, LocalDecls, Terminator};
use rustc_middle::ty::{self, TyKind};
use rustc_index::vec::IndexVec;
use rustc_span::def_id::DefId;

use std::env;

const NEXT_LINE:&str = "\n";
const PADDING:&str = "    ";
const EXPLAIN:&str = " @ ";

type BasicBlocks<'tcx> = IndexVec<BasicBlock, BasicBlockData<'tcx>>;

// In this crate we will costume the display information for Compiler-Metadata by implementing
// rlc::Display for target data structure.
// If you want output these messages that makes debug easily, please add -v or -vv to rlc
// that makes entire rlc verbose.

// MirDisplay is the controller in rlc to determine if the Display trait should be derived.
#[derive(Debug, Copy, Clone, Hash)]
pub enum MirDisplay {
    // Basic MIR information for Debug
    Verbose,
    // MIR associated with the type of statements and terminators, and the types of variables
    VeryVerobse,
    Disabled,
}

pub fn is_display_verbose() -> bool {
    match env::var_os("MIR_DISPLAY") {
        Some(verbose)  => match verbose.as_os_str().to_str().unwrap() {
            "VERBOSE" => true,
            "VERY VERBOSE" => true,
            _ => false,
        },
        _ => false,
    }
}

pub fn is_display_very_verbose() -> bool {
    match env::var_os("MIR_DISPLAY") {
        Some(verbose)  => match verbose.as_os_str().to_str().unwrap() {
            "VERY VERBOSE" => true,
            _ => false,
        },
        _ => false,
    }
}

// This trait is a wrapper towards std::Display or std::Debug, and is to resolve orphan restrictions.
pub trait Display {
    fn display(&self) -> String;
}

impl<'tcx> Display for Terminator<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &format!("{}{:?}{}", PADDING, self.kind, self.kind.display());
        }
        s
    }
}

impl<'tcx> Display for TerminatorKind<'tcx>{
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_very_verbose() {
            s += EXPLAIN;
            match &self {
                TerminatorKind::Goto { .. } =>
                    s += "Goto",
                TerminatorKind::SwitchInt { .. } =>
                    s += "SwitchInt",
                TerminatorKind::Resume =>
                    s += "Resume",
                TerminatorKind::Abort =>
                    s += "Abort",
                TerminatorKind::Return =>
                    s += "Return",
                TerminatorKind::Unreachable =>
                    s += "Unreachable",
                TerminatorKind::Drop { .. } =>
                    s += "Drop",
                TerminatorKind::DropAndReplace { .. } =>
                    s += "DropAndReplace",
                TerminatorKind::Assert { .. } =>
                    s += "Assert",
                TerminatorKind::Yield { .. } =>
                    s += "Yield",
                TerminatorKind::GeneratorDrop =>
                    s += "GeneratorDrop",
                TerminatorKind::FalseEdge { .. } =>
                    s += "FalseEdge",
                TerminatorKind::FalseUnwind { .. } =>
                    s += "FalseUnwind",
                TerminatorKind::InlineAsm { .. } =>
                    s += "InlineAsm",
                TerminatorKind::Call { func, .. } => {
                    match func {
                        Operand::Constant(constant) => {
                                match constant.literal.ty().kind() {
                                    ty::FnDef(id, ..) =>
                                        s += &format!("Call: FnDid: {}", id.index.as_usize()).as_str(),
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

impl<'tcx> Display for Statement<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &format!("{}{:?}{}", PADDING, self.kind, self.kind.display());
        }
        s
    }
}

impl<'tcx> Display for StatementKind<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_very_verbose() {
            s += EXPLAIN;
            match &self {
                StatementKind::Assign(assign) => {
                    s += &format!("{:?}={:?}{}", assign.0, assign.1, assign.1.display());
                }
                StatementKind::FakeRead( .. ) =>
                    s += "FakeRead",
                StatementKind::SetDiscriminant { .. } =>
                    s += "SetDiscriminant",
                StatementKind::Deinit( .. ) =>
                    s += "Deinit",
                StatementKind::StorageLive( .. ) =>
                    s += "StorageLive",
                StatementKind::StorageDead( .. ) =>
                    s += "StorageDead",
                StatementKind::Retag( .. ) =>
                    s += "Retag",
                StatementKind::AscribeUserType( .. ) =>
                    s += "AscribeUserType",
                StatementKind::Coverage( .. ) =>
                    s += "Coverage",
                StatementKind::CopyNonOverlapping( .. ) =>
                    s += "CopyNonOverlapping",
                StatementKind::Nop =>
                    s += "Nop",
            }
        } else {
            ()
        }
        s
    }
}

impl<'tcx> Display for Rvalue<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_very_verbose() {
            s += EXPLAIN;
            match self {
                Rvalue::Use( .. ) =>
                    s += "Use",
                Rvalue::Repeat( .. ) =>
                    s += "Repeat",
                Rvalue::Ref( .. ) =>
                    s += "Ref",
                Rvalue::ThreadLocalRef( .. ) =>
                    s += "ThreadLocalRef",
                Rvalue::AddressOf( .. ) =>
                    s += "AddressOf",
                Rvalue::Len( .. ) =>
                    s += "Len",
                Rvalue::Cast( .. ) =>
                    s += "Cast",
                Rvalue::BinaryOp( .. ) =>
                    s += "BinaryOp",
                Rvalue::CheckedBinaryOp( .. ) =>
                    s += "CheckedBinaryOp",
                Rvalue::NullaryOp( .. ) =>
                    s += "NullaryOp",
                Rvalue::UnaryOp( .. ) =>
                    s += "UnaryOp",
                Rvalue::Discriminant( .. ) =>
                    s += "Discriminant",
                Rvalue::Aggregate( .. ) =>
                    s += "Aggregate",
                Rvalue::ShallowInitBox( .. ) =>
                    s+= "ShallowInitBox",
                Rvalue::CopyForDeref( .. ) =>
                    s+= "CopyForDeref",
            }
        } else {
            ()
        }
        s
    }
}

impl<'tcx> Display for BasicBlocks<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            for (index, bb) in self.iter().enumerate() {
                s += &format!("bb {} {{{}{}}}{}", index, NEXT_LINE, bb.display(), NEXT_LINE);
            }
        }
        s
    }
}

impl<'tcx> Display for BasicBlockData<'tcx>  {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &format!("CleanUp: {}{}", self.is_cleanup, NEXT_LINE);
            for stmt in self.statements.iter() {
                s += &format!("{}{}", stmt.display(), NEXT_LINE);
            }
            s += &format!("{}{}", self.terminator.clone().unwrap().display(), NEXT_LINE);
        }
        s
    }
}

impl<'tcx> Display for LocalDecls<'tcx>  {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            for (index, ld) in self.iter().enumerate() {
                s += &format!("_{}: {} {}", index, ld.display(), NEXT_LINE);
            }
        }
        s
    }
}

impl<'tcx> Display for LocalDecl<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &format!("{:?}", self.ty);
        }
        if is_display_very_verbose() {
            s += &format!("{}{}", EXPLAIN, self.ty.kind().display())
        }
        s
    }
}

impl<'tcx> Display for Body<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &self.local_decls.display();
            s += &self.basic_blocks().display();
        }
        s
    }
}

impl<'tcx> Display for TyKind<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        if is_display_verbose() {
            s += &format!("{:?}", self);
        }
        s
    }
}

impl Display for DefId {
    fn display(&self) -> String {
        format!("{:?}", self)
    }
}