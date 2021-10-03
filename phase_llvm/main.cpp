//
// Created by VaynNecol on 2021/9/22.
// This module is preferred to be used in RLC
// and can be also used in heap cost analysis for new Rust RFC.

// The binary is automated emitted by 'install_rlc.sh' with rlc
// and the cmake file is 'CMakeList.txt'.
// We designed this tool used in Unix-like environment
// (and we do not support windows as your host system

#include <iostream>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IRReader/IRReader.h>
#include <llvm/Support/SourceMgr.h>
#include <llvm/Support/ManagedStatic.h>
#include <llvm/Support/CommandLine.h>
#include "test.h"

// The Global Context in LLVM
static llvm::ManagedStatic<llvm::LLVMContext> GlobalContext;
// The Global CLI Argument for 'main' and the argument is actually the '.ll' file
static llvm::cl::opt<std::string> InputFile(llvm::cl::Positional, llvm::cl::desc("<filename>.ll"), llvm::cl::Required);

// The input of this binary is the dir to llvm-ir file
// and the default dir in unix-like os is '/tmp/rlc-llvm-ir/*.ll'
int main(int argc, char **argv) {

    if (argc == 0) {
        llvm::errs() << "Failed due to lack of input LLVM-IR file for rlc_phase_llvm\n";
        exit(1);
    }

    // Instance of Diagnostic
    llvm::SMDiagnostic Err;
    // Format CLI Argument
    llvm::cl::ParseCommandLineOptions(argc, argv);
    // Read and format llvm-bc file,
    // Return the Module of LLVM
    std::unique_ptr<llvm::Module> M = parseIRFile(InputFile, Err, *GlobalContext);

    // Error Handling for Parsing LLVM-IR
    if (!M) {
        Err.print(argv[0], llvm::errs());
        return 1;
    }

    test_parsing(*M);

}