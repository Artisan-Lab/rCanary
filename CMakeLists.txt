cmake_minimum_required(VERSION 3.5)

# Use 'where clang' to find dir to clang and change CMAKE_C_COMPILER
set(CMAKE_C_COMPILER             "/usr/local/opt/llvm/bin/clang")
set(CMAKE_C_FLAGS                "-Wall -std=c99")
set(CMAKE_C_FLAGS_DEBUG          "-g")
set(CMAKE_C_FLAGS_MINSIZEREL     "-Os -DNDEBUG")
set(CMAKE_C_FLAGS_RELEASE        "-O4 -DNDEBUG")
set(CMAKE_C_FLAGS_RELWITHDEBINFO "-O2 -g")

# Use 'where clang++' to find dir to clang++ and change CMAKE_CXX_COMPILER
set(CMAKE_CXX_COMPILER             "/usr/local/opt/llvm/bin/clang++")
set(CMAKE_CXX_FLAGS                "${CMAKE_CXX_FLAGS} -O3 -std=c++11")
set(CMAKE_CXX_FLAGS_DEBUG          "-g")
set(CMAKE_CXX_FLAGS_MINSIZEREL     "-Os -DNDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE        "-O4 -DNDEBUG")
set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O2 -g")
set(CMAKE_CXX_STANDARD 14)

# Like before to modify llvm - bins
set(CMAKE_AR      "/usr/local/opt/llvm/bin/llvm-ar")
set(CMAKE_LINKER  "/usr/local/opt/llvm/bin/llvm-link")
set(CMAKE_NM      "/usr/local/opt/llvm/bin/llvm-nm")
set(CMAKE_OBJDUMP "/usr/local/opt/llvm/bin/llvm-objdump")
set(CMAKE_RANLIB  "/usr/local/opt/llvm/bin/llvm-ranlib")

#your project name
project(rlc_phase_llvm)

find_package(LLVM REQUIRED CONFIG)

message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")

# Set your project compile flags.
# E.g. if using the C++ header files you will need to enable C++11 support
# for your compiler.
include_directories(${LLVM_INCLUDE_DIRS})
separate_arguments(LLVM_DEFINITIONS_LIST NATIVE_COMMAND ${LLVM_DEFINITIONS})
add_definitions(${LLVM_DEFINITIONS_LIST})

# Now build our tools
# Need to modify the path to main.cpp
add_executable(${PROJECT_NAME} phase_llvm/main.cpp) #to be done

# Find the libraries that correspond to the LLVM components
# that we wish to use
llvm_map_components_to_libnames(llvm_libs support core irreader)

# Link against LLVM libraries
target_link_libraries(${PROJECT_NAME} ${llvm_libs})