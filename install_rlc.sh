#!/bin/zsh
cargo install --debug --path "$(dirname "$0")" --force --features backtraces
# cargo install --path "$(dirname "$0")" --force

cd phase_llvm && mkdir "cmake-build"
cd cmake-build
cmake -DCMAKE_BUILD_TYPE=Debug -DCMAKE_DEPENDS_USE_COMPILER=FALSE -G "CodeBlocks - Unix Makefiles" "../../$(dirname "$0")"
cmake --build "$(dirname "$0")" --target rlc_phase_llvm -v -- -j 9
PAT="export PATH=\"\$PATH:${PWD}/\""
echo $PAT >> ~/.zshrc
export PATH="$PATH:${PWD}"