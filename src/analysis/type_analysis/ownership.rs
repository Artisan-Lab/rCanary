use std::fmt::Debug;
use rustc_middle::ty::Ty;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum    RawTypeOwner {
    Owned,
    Unowned,
    Uninit,
}

impl Default for RawTypeOwner {
    fn default() -> Self {
        Self::Uninit
    }
}

impl RawTypeOwner {
    pub fn is_owned(&self) -> bool {
        match self {
            RawTypeOwner::Owned => true,
            RawTypeOwner::Unowned => false,
            RawTypeOwner::Uninit => false,
        }
    }
}

pub enum TypeOwner<'tcx> {
    Owned(Ty<'tcx>),
    Unowned,
}