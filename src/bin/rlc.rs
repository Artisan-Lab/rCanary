#![feature(rustc_private)]
#![feature(backtrace)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code))]

extern crate rustc_driver;
extern crate rustc_interface;

#[macro_use]
extern crate log as rust_log;

use rustc_driver::{Compilation,Callbacks};
use rustc_interface::{interface::Compiler, Queries};

use std::env;
use std::fmt::{Display, Formatter};

use rlc::{RlcConfig, compile_time_sysroot, RLC_DEFAULT_ARGS, start_analyzer};
use rlc::analysis::flow_analysis::{IcxSliceDisplay, Z3GoalDisplay};
use rlc::analysis::type_analysis::AdtOwnerDisplay;
use rlc::display::MirDisplay;
use rlc::grain::RlcGrain;
use rlc::log::Verbosity;
use rlc::rlc_info;

#[derive(Copy, Clone)]
struct RlcCompilerCalls {
    rlc_config: RlcConfig,
}

impl Default for RlcCompilerCalls {
    fn default() -> Self { Self { rlc_config: RlcConfig::default() } }
}

impl Display for RlcCompilerCalls {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.rlc_config.grain(),
        )
    }
}

impl Callbacks for RlcCompilerCalls {
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        compiler.session().abort_if_errors();
        Verbosity::init_rlc_log_system_with_verbosity(self.rlc_config.verbose()).expect("Failed to set up RLC log system");

        rlc_info!("RLC Start");
        queries.global_ctxt().unwrap().peek_mut().enter(
            |tcx| start_analyzer(tcx, self.rlc_config)
        );
        rlc_info!("RLC Stop");

        compiler.session().abort_if_errors();
        Compilation::Continue
    }
}

impl RlcCompilerCalls {
    #[allow(dead_code)]
    fn new(rlc_config: RlcConfig) -> Self { Self {rlc_config} }
}

struct RlcArgs {
    rlc_cc: RlcCompilerCalls,
    args: Vec<String>,
}

impl Default for RlcArgs {
    fn default() -> Self {
        Self {
            rlc_cc: RlcCompilerCalls::default(),
            args: vec![],
        }
    }
}

impl Display for RlcArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Whole Args: {:?}", self.rlc_cc, self.args)
    }
}

impl RlcArgs {
    pub fn set_config_low(&mut self) { self.rlc_cc.rlc_config.set_grain(RlcGrain::Low); }

    pub fn set_config_medium(&mut self) { self.rlc_cc.rlc_config.set_grain(RlcGrain::Medium); }

    pub fn set_config_high(&mut self) { self.rlc_cc.rlc_config.set_grain(RlcGrain::High); }

    pub fn set_config_ultra(&mut self) { self.rlc_cc.rlc_config.set_grain(RlcGrain::Ultra); }

    pub fn set_mir_display_verbose(&mut self) {self.rlc_cc.rlc_config.set_mir_display(MirDisplay::Verbose); }

    pub fn set_mir_display_very_verbose(&mut self) { self.rlc_cc.rlc_config.set_mir_display(MirDisplay::VeryVerobse); }

    pub fn set_adt_display_verbose(&mut self) { self.rlc_cc.rlc_config.set_adt_display(AdtOwnerDisplay::Verbose); }

    pub fn set_z3_goal_display_verbose(&mut self) { self.rlc_cc.rlc_config.set_z3_goal_display(Z3GoalDisplay::Verbose); }

    pub fn set_icx_slice_display(&mut self) { self.rlc_cc.rlc_config.set_icx_slice_display(IcxSliceDisplay::Verbose); }

    pub fn push_args(&mut self, arg: String) { self.args.push(arg); }

    pub fn splice_args(&mut self) {
        self.args.splice(1..1, RLC_DEFAULT_ARGS.iter().map(ToString::to_string));
    }
}

fn config_parse() -> RlcArgs {
    let mut rlc_args = RlcArgs::default();
    for arg in env::args() {
        match arg.as_str() {
            "-GRAIN=LOW" => rlc_args.set_config_low(),
            "-GRAIN=MEDIUM" => rlc_args.set_config_medium(),
            "-GRAIN=HIGH" => rlc_args.set_config_high(),
            "-GRAIN=ULTRA" => rlc_args.set_config_ultra(),
            "-MIR=V" => rlc_args.set_mir_display_verbose(),
            "-MIR=VV" => rlc_args.set_mir_display_very_verbose(),
            "-ADT=V" => rlc_args.set_adt_display_verbose(),
            "-Z3-GOAL=V" => rlc_args.set_z3_goal_display_verbose(),
            "-ICX-SLICE=V" => rlc_args.set_icx_slice_display(),
            _ => rlc_args.push_args(arg),
        }
    }
    rlc_args
}

/// Execute a compiler with the given CLI arguments and callbacks.
fn run_complier(rlc_args: &mut RlcArgs) -> i32 {
    // Make sure we use the right default sysroot. The default sysroot is wrong,
    // because `get_or_default_sysroot` in `librustc_session` bases that on `current_exe`.
    //
    // Make sure we always call `compile_time_sysroot` as that also does some sanity-checks
    // of the environment we were built in.
    // FIXME: Ideally we'd turn a bad build env into a compile-time error via CTFE or so.
    if let Some(sysroot) = compile_time_sysroot() {
        let sysroot_flag = "--sysroot";
        if !rlc_args.args.iter().any(|e| e == sysroot_flag) {
            // We need to overwrite the default that librustc_session would compute.
            rlc_args.push_args(sysroot_flag.to_owned());
            rlc_args.push_args(sysroot);
        }
    }
    // Finally, add the default flags all the way in the beginning, but after the binary name.
    rlc_args.splice_args();

    let rlc_final_args = rlc_args.args.clone();

    let run_compiler = rustc_driver::RunCompiler::new(&rlc_args.args, &mut rlc_args.rlc_cc);
    let exit_code = rustc_driver::catch_with_exit_code(move || run_compiler.run());

    if option_env!("RLC_VERBOSE").is_some() {
        rlc_info!("The arg for compilation is {:?}", rlc_final_args);
    }

    exit_code
}

fn main() {
    // Installs a panic hook that will print the ICE message on unexpected panics.
    rustc_driver::install_ice_hook();

    // Parse the config and arguments from env.
    let mut rlc_args = config_parse();

    if env::var_os("RUSTC_LOG").is_some() {
        rustc_driver::init_rustc_env_logger();
    }

    debug!("RLC-Args: {}", &rlc_args);

    let exit_code = run_complier(&mut rlc_args);
    std::process::exit(exit_code)
}