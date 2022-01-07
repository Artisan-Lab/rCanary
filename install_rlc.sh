#!/bin/zsh

echo "Building RLC and install RLC by Cargo"
#cargo clean
#for debug version
#cargo install --debug --path "$(dirname "$0")" --force --features backtraces
#for release version
cargo install --path "$(dirname "$0")" --force

echo "Building RLC_Phase_LLVM by CMake and add the tool into Environment"
cd phase_llvm || exit
mkdir "cmake-build"
cd cmake-build || exit
cmake -DCMAKE_BUILD_TYPE=Debug -DCMAKE_DEPENDS_USE_COMPILER=FALSE -G "CodeBlocks - Unix Makefiles" "../../$(dirname "$0")"
cmake --build "$(dirname "$0")" --target rlc_phase_llvm -v -- -j 9
p="export PATH=\"\$PATH:${PWD}/\""
echo $p >> ~/.zshrc
#echo $p >> ~/.bashrc
export PATH="$PATH:${PWD}"