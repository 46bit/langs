use super::*;
use llvm;
use llvm::prelude::*;
use std::collections::HashMap;

pub struct ExpressionSynthesiser<'a> {
    llvm_ctx: LLVMContextRef,
    llvm_builder: LLVMBuilderRef,
    vars: &'a HashMap<Name, Var>,
    fns: &'a HashMap<Name, LLVMValueRef>,
}

impl<'a> ExpressionSynthesiser<'a> {
    pub unsafe fn synthesise(
        llvm_ctx: LLVMContextRef,
        llvm_builder: LLVMBuilderRef,
        expression: &Expression,
        vars: &'a HashMap<Name, Var>,
        fns: &HashMap<Name, LLVMValueRef>,
    ) -> LLVMValueRef {
        ExpressionSynthesiser {
            llvm_ctx: llvm_ctx,
            llvm_builder: llvm_builder,
            vars: vars,
            fns: fns,
        }.synthesise_expression(expression)
    }

    unsafe fn synthesise_expression(&self, expression: &Expression) -> LLVMValueRef {
        match expression {
            &Expression::Operand(ref operand) => self.synthesise_operand(operand),
            &Expression::Operation(operator, ref lhs, ref rhs) => {
                self.synthesise_operation(operator, lhs, rhs)
            }
        }
    }

    unsafe fn synthesise_operation(
        &self,
        operator: Operator,
        lhs: &Expression,
        rhs: &Expression,
    ) -> LLVMValueRef {
        let llvm_lhs = self.synthesise_expression(lhs);
        let llvm_rhs = self.synthesise_expression(rhs);
        let llvm_name = llvm_name("tmp");
        match operator {
            Operator::Subtract => {
                llvm::core::LLVMBuildSub(self.llvm_builder, llvm_lhs, llvm_rhs, llvm_name.as_ptr())
            }
            Operator::Add => {
                llvm::core::LLVMBuildAdd(self.llvm_builder, llvm_lhs, llvm_rhs, llvm_name.as_ptr())
            }
            Operator::Divide => {
                llvm::core::LLVMBuildSDiv(self.llvm_builder, llvm_lhs, llvm_rhs, llvm_name.as_ptr())
            }
            Operator::Multiply => {
                llvm::core::LLVMBuildMul(self.llvm_builder, llvm_lhs, llvm_rhs, llvm_name.as_ptr())
            }
        }
    }

    unsafe fn synthesise_operand(&self, operand: &Operand) -> LLVMValueRef {
        let i64_type = llvm::core::LLVMInt64TypeInContext(self.llvm_ctx);
        match operand {
            &Operand::I64(n) => llvm::core::LLVMConstInt(i64_type, n as u64, 0),
            &Operand::Group(ref expression) => self.synthesise_expression(expression),
            &Operand::VarSubstitution(ref name) => {
                self.vars[name].synthesise_substitution(self.llvm_builder, name)
            }
            &Operand::FnApplication(ref name, ref args) => synthesise_fn_application(
                self.llvm_builder,
                name,
                args.iter().map(|e| self.synthesise_expression(e)).collect(),
                self.fns,
            ),
        }
    }
}