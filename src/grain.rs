use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum RlcGrain {
    Low = 0,
    Medium = 1,
    High = 2,
    Ultra = 3,
}

impl Display for RlcGrain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RlcGrain::Ultra => "Ultra",
                RlcGrain::High => "High",
                RlcGrain::Medium => "Medium",
                RlcGrain::Low => "Low,"
            }
        )
    }
}