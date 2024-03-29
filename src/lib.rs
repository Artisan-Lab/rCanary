#![feature(rustc_private)]
#![feature(backtrace)]
#![feature(control_flow_enum)]
#![feature(box_patterns)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code, unused_must_use))]
#![cfg_attr(not(debug_assertions), allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code, unused_must_use))]

pub mod analysis;
pub mod components;

extern crate rustc_middle;
extern crate rustc_hir;
extern crate rustc_span;
extern crate rustc_index;
extern crate rustc_target;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
extern crate core;

use rustc_middle::ty::TyCtxt;

use crate::components::grain::RlcGrain;
use crate::components::log::Verbosity;
use crate::components::context::RlcGlobalCtxt;
use crate::components::display::MirDisplay;
use crate::analysis::flow_analysis::{FlowAnalysis, IcxSliceDisplay, Z3GoalDisplay};
use crate::analysis::type_analysis::{TypeAnalysis, AdtOwnerDisplay};

// Insert rustc arguments at the beginning of the argument list that RLC wants to be
// set per default, for maximal validation power.
pub static RLC_DEFAULT_ARGS: &[&str] =
    &["-Zalways-encode-mir", "-Zmir-opt-level=0", "--cfg=rlc"];
pub static RLC_ROOT:&str = "/tmp/rlc";
pub static RLC_LLVM_CACHE:&str = "/tmp/rlc/llvm-cache";
pub static RLC_LLVM_IR:&str = "/tmp/rlc/llvm-ir";
pub static RLC_LLVM_RES:&str = "/tmp/rlc/llvm-res";

pub type Elapsed = (i64, i64);

#[derive(Debug, Copy, Clone, Hash)]
pub struct RlcConfig {
    grain: RlcGrain,
    verbose: Verbosity,
    mir_display: MirDisplay,
    adt_display: AdtOwnerDisplay,
    z3_goal_display: Z3GoalDisplay,
    icx_slice_display: IcxSliceDisplay,
}

impl Default for RlcConfig {
    fn default() -> Self {
        Self {
            grain: RlcGrain::Low,
            verbose: Verbosity::Info,
            mir_display: MirDisplay::Disabled,
            adt_display: AdtOwnerDisplay::Disabled,
            z3_goal_display: Z3GoalDisplay::Disabled,
            icx_slice_display: IcxSliceDisplay::Disabled,
        }
    }
}

impl RlcConfig {
    pub fn new(
        grain: RlcGrain,
        verbose: Verbosity,
        mir_display: MirDisplay,
        adt_display: AdtOwnerDisplay,
        z3_goal_display: Z3GoalDisplay,
        icx_slice_display: IcxSliceDisplay,
    ) -> Self {
        Self {
            grain,
            verbose,
            mir_display,
            adt_display,
            z3_goal_display,
            icx_slice_display,
        }
    }

    pub fn grain(&self) -> RlcGrain { self.grain }

    pub fn set_grain(&mut self, grain: RlcGrain) { self.grain = grain;}

    pub fn verbose(&self) -> Verbosity { self.verbose }

    pub fn set_verbose(&mut self, verbose: Verbosity) { self.verbose = verbose; }

    pub fn mir_display(&self) -> MirDisplay { self.mir_display }

    pub fn set_mir_display(&mut self, mir_display: MirDisplay) { self.mir_display = mir_display; }

    pub fn adt_display(&self) -> AdtOwnerDisplay { self.adt_display }

    pub fn set_adt_display(&mut self, adt_display: AdtOwnerDisplay) { self.adt_display = adt_display; }

    pub fn z3_goal_display(&self) -> Z3GoalDisplay { self.z3_goal_display }

    pub fn set_z3_goal_display(&mut self, z3_goal_display: Z3GoalDisplay) { self.z3_goal_display = z3_goal_display; }

    pub fn icx_slice_display(&self) -> IcxSliceDisplay { self.icx_slice_display }

    pub fn set_icx_slice_display(&mut self, icx_slice_display: IcxSliceDisplay) { self.icx_slice_display = icx_slice_display; }

}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum RlcPhase {
    PreProcess,
    LLVM,
    Cargo,
    Rustc,
}

/// Returns the "default sysroot" that RLC will use if no `--sysroot` flag is set.
/// Should be a compile-time constant.
pub fn compile_time_sysroot() -> Option<String> {
    // Optionally inspects an environment variable at compile time.
    if option_env!("RUSTC_STAGE").is_some() {
        // This is being built as part of rustc, and gets shipped with rustup.
        // We can rely on the sysroot computation in rustc.
        return None;
    }

    // For builds outside rustc, we need to ensure that we got a sysroot
    // that gets used as a default.  The sysroot computation in librustc_session would
    // end up somewhere in the build dir (see `get_or_default_sysroot`).
    // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    let env = if home.is_some() && toolchain.is_some() {
         format!("{}/toolchains/{}", home.unwrap(), toolchain.unwrap())
    } else {
        option_env!("RUST_SYSROOT")
            .expect("To build RLC without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_string()
    };

    Some(env)
}

fn run_analyzer<F, R>(name: &str, func: F) -> R
    where F: FnOnce() -> R
{
    rlc_info!("{} Start", name);
    let res = func();
    rlc_info!("{} Done", name);
    res
}

pub fn start_analyzer(tcx: TyCtxt, config: RlcConfig) {
    let rcx_boxed = Box::new(RlcGlobalCtxt::new(tcx, config));
    let rcx = Box::leak(rcx_boxed);

    run_analyzer(
        "Type Analysis",
        ||
            TypeAnalysis::new(rcx).start()
    );

    run_analyzer(
        "Flow Analysis",
        ||
            FlowAnalysis::new(rcx).start()
    );
}