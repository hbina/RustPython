use super::{JitCompileError, JitType};
use num_traits::cast::ToPrimitive;
use rustpython_compiler_core::bytecode::{
    self, BinaryOperator, BorrowedConstant, CodeObject, ComparisonOperator, Instruction, Label,
    OpArg, OpArgState,
};
use std::collections::BTreeSet;

/// Represents a value on the simulated stack during C code generation
#[derive(Clone, Debug)]
struct CValue {
    /// The C expression representing this value
    expr: String,
    /// The type of this value
    ty: JitType,
}

/// Represents a local variable in the generated C code
#[derive(Clone, Debug)]
struct CVariable {
    /// The C variable name
    name: String,
    /// The type of this variable
    ty: JitType,
}

/// C code generator that translates Python bytecode to C source code
pub struct CCodeGenerator {
    /// The generated C code (function body)
    body: String,
    /// Simulated value stack
    stack: Vec<CValue>,
    /// Local variables (indexed by bytecode variable index)
    variables: Vec<Option<CVariable>>,
    /// Counter for generating unique temporary variable names
    temp_counter: usize,
    /// Set of labels that are jump targets
    label_targets: BTreeSet<Label>,
    /// Function argument types
    arg_types: Vec<JitType>,
    /// Function return type
    ret_type: Option<JitType>,
    /// Function name
    func_name: String,
    /// Argument names from bytecode
    arg_names: Vec<String>,
    /// Whether we need the ipow helper function
    needs_ipow: bool,
    /// Whether we need the math.h include for pow()
    needs_math: bool,
    /// Current indentation level
    indent: usize,
}

impl CCodeGenerator {
    /// Create a new C code generator
    pub fn new<C: bytecode::Constant>(
        bytecode: &CodeObject<C>,
        arg_types: &[JitType],
        ret_type: Option<JitType>,
    ) -> Result<Self, JitCompileError> {
        let num_variables = bytecode.varnames.len();
        let func_name = bytecode.obj_name.as_ref().to_string();
        let arg_names: Vec<String> = bytecode
            .varnames
            .iter()
            .take(arg_types.len())
            .map(|s| s.as_ref().to_string())
            .collect();

        let label_targets = bytecode.label_targets();

        let mut generator = CCodeGenerator {
            body: String::new(),
            stack: Vec::new(),
            variables: vec![None; num_variables],
            temp_counter: 0,
            label_targets,
            arg_types: arg_types.to_vec(),
            ret_type,
            func_name,
            arg_names,
            needs_ipow: false,
            needs_math: false,
            indent: 1,
        };

        // Initialize variables for function arguments
        for (i, ty) in arg_types.iter().enumerate() {
            let name = generator
                .arg_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("arg_{}", i));
            generator.variables[i] = Some(CVariable {
                name,
                ty: ty.clone(),
            });
        }

        Ok(generator)
    }

    /// Generate a unique temporary variable name
    fn new_temp(&mut self, _ty: &JitType) -> String {
        let name = format!("tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    /// Write an indented line to the body
    fn writeln(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.body.push_str("    ");
        }
        self.body.push_str(line);
        self.body.push('\n');
    }

    /// Pop a value from the stack
    fn pop(&mut self) -> Result<CValue, JitCompileError> {
        self.stack.pop().ok_or(JitCompileError::BadBytecode)
    }

    /// Push a value onto the stack
    fn push(&mut self, value: CValue) {
        self.stack.push(value);
    }

    /// Store a value in a local variable
    fn store_variable(&mut self, idx: u32, value: CValue) -> Result<(), JitCompileError> {
        let idx = idx as usize;

        // Check if this is an argument (already has a variable)
        if idx < self.arg_types.len() {
            // Re-assigning to an argument - need to emit assignment
            let var = self.variables[idx]
                .as_ref()
                .ok_or(JitCompileError::BadBytecode)?;
            if var.ty != value.ty {
                return Err(JitCompileError::TypeMismatch);
            }
            self.writeln(&format!("{} = {};", var.name, value.expr));
        } else {
            // New local variable
            let var_name = format!("var_{}", idx);

            if let Some(existing) = &self.variables[idx] {
                // Variable already exists, just assign
                if existing.ty != value.ty {
                    return Err(JitCompileError::TypeMismatch);
                }
                self.writeln(&format!("{} = {};", existing.name, value.expr));
            } else {
                // New variable - declare and assign
                self.writeln(&format!(
                    "{} {} = {};",
                    value.ty.to_c_type(),
                    var_name,
                    value.expr
                ));
                self.variables[idx] = Some(CVariable {
                    name: var_name,
                    ty: value.ty,
                });
            }
        }
        Ok(())
    }

    /// Load a local variable onto the stack
    fn load_variable(&mut self, idx: u32) -> Result<(), JitCompileError> {
        let var = self.variables[idx as usize]
            .as_ref()
            .ok_or(JitCompileError::BadBytecode)?;
        self.push(CValue {
            expr: var.name.clone(),
            ty: var.ty.clone(),
        });
        Ok(())
    }

    /// Compile bytecode to C
    pub fn compile<C: bytecode::Constant>(
        &mut self,
        bytecode: &CodeObject<C>,
    ) -> Result<(), JitCompileError> {
        let mut arg_state = OpArgState::default();

        for (offset, &raw_instr) in bytecode.instructions.iter().enumerate() {
            let label = Label(offset as u32);
            let (instruction, arg) = arg_state.get(raw_instr);

            // If this is a jump target, emit a label
            if self.label_targets.contains(&label) {
                // Reduce indent for label
                self.body.push_str(&format!("label_{}:\n", offset));
            }

            self.compile_instruction(bytecode, instruction, arg)?;
        }

        Ok(())
    }

    /// Compile a single instruction
    fn compile_instruction<C: bytecode::Constant>(
        &mut self,
        bytecode: &CodeObject<C>,
        instruction: Instruction,
        arg: OpArg,
    ) -> Result<(), JitCompileError> {
        match instruction {
            Instruction::LoadConst { idx } => {
                let value = self
                    .prepare_const(bytecode.constants[idx.get(arg) as usize].borrow_constant())?;
                self.push(value);
            }

            Instruction::LoadFast(idx) => {
                self.load_variable(idx.get(arg))?;
            }

            Instruction::StoreFast(idx) => {
                let value = self.pop()?;
                self.store_variable(idx.get(arg), value)?;
            }

            Instruction::BinaryOp { op } => {
                let op = op.get(arg);
                let b = self.pop()?;
                let a = self.pop()?;
                self.compile_binary_op(op, a, b)?;
            }

            Instruction::CompareOperation { op, .. } => {
                let op = op.get(arg);
                let b = self.pop()?;
                let a = self.pop()?;
                self.compile_compare_op(op, a, b)?;
            }

            Instruction::UnaryNegative => {
                let a = self.pop()?;
                let temp = self.new_temp(&a.ty);
                let c_type = a.ty.to_c_type();
                self.writeln(&format!("{} {} = -{};", c_type, temp, a.expr));
                self.push(CValue {
                    expr: temp,
                    ty: a.ty,
                });
            }

            Instruction::UnaryNot => {
                let a = self.pop()?;
                let temp = self.new_temp(&JitType::Bool);
                self.writeln(&format!("int {} = !{};", temp, a.expr));
                self.push(CValue {
                    expr: temp,
                    ty: JitType::Bool,
                });
            }

            Instruction::ToBool => {
                let a = self.pop()?;
                let temp = self.new_temp(&JitType::Bool);
                let expr = match a.ty {
                    JitType::Int => format!("{} != 0", a.expr),
                    JitType::Float => format!("{} != 0.0", a.expr),
                    JitType::Bool => a.expr.clone(),
                };
                self.writeln(&format!("int {} = {};", temp, expr));
                self.push(CValue {
                    expr: temp,
                    ty: JitType::Bool,
                });
            }

            Instruction::ReturnValue => {
                let value = self.pop()?;
                self.writeln(&format!("return {};", value.expr));
            }

            Instruction::ReturnConst { idx } => {
                let value = self
                    .prepare_const(bytecode.constants[idx.get(arg) as usize].borrow_constant())?;
                self.writeln(&format!("return {};", value.expr));
            }

            Instruction::Jump { target } => {
                let target_label = target.get(arg);
                self.writeln(&format!("goto label_{};", target_label.0));
            }

            Instruction::PopJumpIfFalse { target } => {
                let cond = self.pop()?;
                let target_label = target.get(arg);
                let cond_expr = self.to_bool_expr(&cond);
                self.writeln(&format!(
                    "if (!{}) goto label_{};",
                    cond_expr, target_label.0
                ));
            }

            Instruction::PopJumpIfTrue { target } => {
                let cond = self.pop()?;
                let target_label = target.get(arg);
                let cond_expr = self.to_bool_expr(&cond);
                self.writeln(&format!(
                    "if ({}) goto label_{};",
                    cond_expr, target_label.0
                ));
            }

            Instruction::PopTop => {
                self.pop()?;
            }

            Instruction::Nop
            | Instruction::Resume { .. }
            | Instruction::ExtendedArg
            | Instruction::PopBlock => {
                // No-op
            }

            Instruction::Swap { index } => {
                let len = self.stack.len();
                let i = len - 1;
                let j = len - 1 - index.get(arg) as usize;
                self.stack.swap(i, j);
            }

            Instruction::CopyItem { index } => {
                let len = self.stack.len();
                let idx = len - 1 - index.get(arg) as usize;
                let value = self.stack[idx].clone();
                self.push(value);
            }

            _ => return Err(JitCompileError::NotSupported),
        }
        Ok(())
    }

    /// Convert a value to a boolean expression string
    fn to_bool_expr(&self, value: &CValue) -> String {
        match value.ty {
            JitType::Bool => value.expr.clone(),
            JitType::Int => format!("({} != 0)", value.expr),
            JitType::Float => format!("({} != 0.0)", value.expr),
        }
    }

    /// Prepare a constant value
    fn prepare_const<C: bytecode::Constant>(
        &mut self,
        constant: BorrowedConstant<'_, C>,
    ) -> Result<CValue, JitCompileError> {
        match constant {
            BorrowedConstant::Integer { value } => {
                let val = value.to_i64().ok_or(JitCompileError::UnsupportedConstant)?;
                Ok(CValue {
                    expr: format!("{}LL", val),
                    ty: JitType::Int,
                })
            }
            BorrowedConstant::Float { value } => Ok(CValue {
                expr: format!("{:.17}", value),
                ty: JitType::Float,
            }),
            BorrowedConstant::Boolean { value } => Ok(CValue {
                expr: if value {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
                ty: JitType::Bool,
            }),
            _ => Err(JitCompileError::UnsupportedConstant),
        }
    }

    /// Compile a binary operation
    fn compile_binary_op(
        &mut self,
        op: BinaryOperator,
        a: CValue,
        b: CValue,
    ) -> Result<(), JitCompileError> {
        let (result_ty, expr) = match (op, &a.ty, &b.ty) {
            // Integer operations
            (BinaryOperator::Add | BinaryOperator::InplaceAdd, JitType::Int, JitType::Int) => {
                (JitType::Int, format!("{} + {}", a.expr, b.expr))
            }
            (
                BinaryOperator::Subtract | BinaryOperator::InplaceSubtract,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} - {}", a.expr, b.expr)),
            (
                BinaryOperator::Multiply | BinaryOperator::InplaceMultiply,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} * {}", a.expr, b.expr)),
            (
                BinaryOperator::FloorDivide | BinaryOperator::InplaceFloorDivide,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} / {}", a.expr, b.expr)),
            (
                BinaryOperator::TrueDivide | BinaryOperator::InplaceTrueDivide,
                JitType::Int,
                JitType::Int,
            ) => (
                JitType::Float,
                format!("(double){} / (double){}", a.expr, b.expr),
            ),
            (
                BinaryOperator::Remainder | BinaryOperator::InplaceRemainder,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} % {}", a.expr, b.expr)),
            (BinaryOperator::Power | BinaryOperator::InplacePower, JitType::Int, JitType::Int) => {
                self.needs_ipow = true;
                (JitType::Int, format!("__jit_ipow({}, {})", a.expr, b.expr))
            }
            (
                BinaryOperator::Lshift | BinaryOperator::InplaceLshift,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} << {}", a.expr, b.expr)),
            (
                BinaryOperator::Rshift | BinaryOperator::InplaceRshift,
                JitType::Int,
                JitType::Int,
            ) => (JitType::Int, format!("{} >> {}", a.expr, b.expr)),
            (BinaryOperator::And | BinaryOperator::InplaceAnd, JitType::Int, JitType::Int) => {
                (JitType::Int, format!("{} & {}", a.expr, b.expr))
            }
            (BinaryOperator::Or | BinaryOperator::InplaceOr, JitType::Int, JitType::Int) => {
                (JitType::Int, format!("{} | {}", a.expr, b.expr))
            }
            (BinaryOperator::Xor | BinaryOperator::InplaceXor, JitType::Int, JitType::Int) => {
                (JitType::Int, format!("{} ^ {}", a.expr, b.expr))
            }

            // Float operations
            (BinaryOperator::Add | BinaryOperator::InplaceAdd, JitType::Float, JitType::Float) => {
                (JitType::Float, format!("{} + {}", a.expr, b.expr))
            }
            (
                BinaryOperator::Subtract | BinaryOperator::InplaceSubtract,
                JitType::Float,
                JitType::Float,
            ) => (JitType::Float, format!("{} - {}", a.expr, b.expr)),
            (
                BinaryOperator::Multiply | BinaryOperator::InplaceMultiply,
                JitType::Float,
                JitType::Float,
            ) => (JitType::Float, format!("{} * {}", a.expr, b.expr)),
            (
                BinaryOperator::TrueDivide | BinaryOperator::InplaceTrueDivide,
                JitType::Float,
                JitType::Float,
            ) => (JitType::Float, format!("{} / {}", a.expr, b.expr)),
            (
                BinaryOperator::Power | BinaryOperator::InplacePower,
                JitType::Float,
                JitType::Float,
            ) => {
                self.needs_math = true;
                (JitType::Float, format!("pow({}, {})", a.expr, b.expr))
            }

            // Mixed int/float operations - promote to float
            (BinaryOperator::Add | BinaryOperator::InplaceAdd, JitType::Int, JitType::Float) => {
                (JitType::Float, format!("(double){} + {}", a.expr, b.expr))
            }
            (BinaryOperator::Add | BinaryOperator::InplaceAdd, JitType::Float, JitType::Int) => {
                (JitType::Float, format!("{} + (double){}", a.expr, b.expr))
            }
            (
                BinaryOperator::Subtract | BinaryOperator::InplaceSubtract,
                JitType::Int,
                JitType::Float,
            ) => (JitType::Float, format!("(double){} - {}", a.expr, b.expr)),
            (
                BinaryOperator::Subtract | BinaryOperator::InplaceSubtract,
                JitType::Float,
                JitType::Int,
            ) => (JitType::Float, format!("{} - (double){}", a.expr, b.expr)),
            (
                BinaryOperator::Multiply | BinaryOperator::InplaceMultiply,
                JitType::Int,
                JitType::Float,
            ) => (JitType::Float, format!("(double){} * {}", a.expr, b.expr)),
            (
                BinaryOperator::Multiply | BinaryOperator::InplaceMultiply,
                JitType::Float,
                JitType::Int,
            ) => (JitType::Float, format!("{} * (double){}", a.expr, b.expr)),
            (
                BinaryOperator::TrueDivide | BinaryOperator::InplaceTrueDivide,
                JitType::Int,
                JitType::Float,
            ) => (JitType::Float, format!("(double){} / {}", a.expr, b.expr)),
            (
                BinaryOperator::TrueDivide | BinaryOperator::InplaceTrueDivide,
                JitType::Float,
                JitType::Int,
            ) => (JitType::Float, format!("{} / (double){}", a.expr, b.expr)),
            (
                BinaryOperator::Power | BinaryOperator::InplacePower,
                JitType::Int,
                JitType::Float,
            ) => {
                self.needs_math = true;
                (
                    JitType::Float,
                    format!("pow((double){}, {})", a.expr, b.expr),
                )
            }
            (
                BinaryOperator::Power | BinaryOperator::InplacePower,
                JitType::Float,
                JitType::Int,
            ) => {
                self.needs_math = true;
                (
                    JitType::Float,
                    format!("pow({}, (double){})", a.expr, b.expr),
                )
            }

            _ => return Err(JitCompileError::NotSupported),
        };

        let temp = self.new_temp(&result_ty);
        let c_type = result_ty.to_c_type();
        self.writeln(&format!("{} {} = {};", c_type, temp, expr));
        self.push(CValue {
            expr: temp,
            ty: result_ty,
        });
        Ok(())
    }

    /// Compile a comparison operation
    fn compile_compare_op(
        &mut self,
        op: ComparisonOperator,
        a: CValue,
        b: CValue,
    ) -> Result<(), JitCompileError> {
        let op_str = match op {
            ComparisonOperator::Less => "<",
            ComparisonOperator::Greater => ">",
            ComparisonOperator::Equal => "==",
            ComparisonOperator::NotEqual => "!=",
            ComparisonOperator::LessOrEqual => "<=",
            ComparisonOperator::GreaterOrEqual => ">=",
        };

        let temp = self.new_temp(&JitType::Bool);
        self.writeln(&format!("int {} = {} {} {};", temp, a.expr, op_str, b.expr));
        self.push(CValue {
            expr: temp,
            ty: JitType::Bool,
        });
        Ok(())
    }

    /// Generate the complete C code
    pub fn into_code(self) -> String {
        let mut output = String::new();

        // Header includes
        output.push_str("#include <stdint.h>\n");
        if self.needs_math {
            output.push_str("#include <math.h>\n");
        }
        output.push('\n');

        // Helper functions
        if self.needs_ipow {
            output.push_str("static int64_t __jit_ipow(int64_t base, int64_t exp) {\n");
            output.push_str("    if (exp < 0) return 0;\n");
            output.push_str("    int64_t result = 1;\n");
            output.push_str("    while (exp > 0) {\n");
            output.push_str("        if (exp & 1) result *= base;\n");
            output.push_str("        base *= base;\n");
            output.push_str("        exp >>= 1;\n");
            output.push_str("    }\n");
            output.push_str("    return result;\n");
            output.push_str("}\n\n");
        }

        // Function signature
        let ret_type = self
            .ret_type
            .as_ref()
            .map(|t| t.to_c_type())
            .unwrap_or("void");

        let params: Vec<String> = self
            .arg_types
            .iter()
            .zip(&self.arg_names)
            .map(|(ty, name)| format!("{} {}", ty.to_c_type(), name))
            .collect();

        output.push_str(&format!(
            "{} {}({}) {{\n",
            ret_type,
            self.func_name,
            params.join(", ")
        ));

        // Function body
        output.push_str(&self.body);

        output.push_str("}\n");

        output
    }
}
