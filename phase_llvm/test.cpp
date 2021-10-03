//
// Created by VaynNecol on 2021/9/27.
//

#include <llvm/IR/Module.h>

void test_parsing(llvm::Module &m) {
    for (llvm::Function &F:m) {
        // 过滤掉那些以llvm.开头的无关函数
        if (!F.isIntrinsic()) {
            // 打印函数返回类型
            llvm::outs() << *(F.getReturnType());
            // 打印函数名
            llvm::outs() << ' ' << F.getName() << '(';
            // 遍历函数的每一个参数g
            for (llvm::Function::arg_iterator it = F.arg_begin(), ie = F.arg_end(); it != ie; it++) {
                // 打印参数类型
                llvm::outs() << *(it->getType());
                if (it != ie - 1) {
                    llvm::outs() << ", ";
                }
            }
            llvm::outs() << ")\n";
        }
    }
}