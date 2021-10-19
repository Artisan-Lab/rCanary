#![feature(rustc_private)]
#![feature(backtrace)]

extern crate rustc_middle;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_span;
extern crate rustc_index;

use rustc_middle::ty::TyCtxt;

use crate::grain::RlcGrain;
use crate::log::Verbosity;
use crate::context::RlcGlobalCtxt;
use crate::type_collector::TypeCollector;

pub mod grain;
pub mod log;
pub mod context;
pub mod display;
pub mod type_collector;

// Insert rustc arguments at the beginning of the argument list that RLC wants to be
// set per default, for maximal validation power.
pub static RLC_DEFAULT_ARGS: &[&str] =
    &["-Zalways-encode-mir", "-Zmir-opt-level=0", "--cfg=rlc"];
pub static RLC_ROOT:&str = "/tmp/rlc";
pub static RLC_LLVM_CACHE:&str = "/tmp/rlc/rlc-llvm-cache";
pub static RLC_LLVM_IR:&str = "/tmp/rlc/rlc-llvm-ir";

#[derive(Debug, Copy, Clone, Hash)]
pub struct RlcConfig {
    grain: RlcGrain,
    verbose: Verbosity,
}

impl Default for RlcConfig {
    fn default() -> Self {
        Self {
            grain: RlcGrain::Low,
            verbose: Verbosity::Info,
        }
    }
}

impl RlcConfig {
    pub fn new(grain: RlcGrain, verbose: Verbosity) -> Self {
        Self {
            grain,
            verbose,
        }
    }

    pub fn grain(&self) -> RlcGrain { self.grain }

    pub fn set_grain(&mut self, grain: RlcGrain) { self.grain = grain;}

    pub fn verbose(&self) -> Verbosity { self.verbose }

    pub fn set_verbose(&mut self, verbose: Verbosity) { self.verbose = verbose; }

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
    let rcx = &*Box::leak(rcx_boxed);

    run_analyzer(
        "Type Collector",
        ||
                TypeCollector::new(&rcx).start()
    );



}