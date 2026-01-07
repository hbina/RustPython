mod instructions;

use instructions::CCodeGenerator;
use rustpython_compiler_core::bytecode;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JitCompileError {
    #[error("function can't be jitted: unsupported instruction")]
    NotSupported,
    #[error("bad bytecode")]
    BadBytecode,
    #[error("unsupported constant type")]
    UnsupportedConstant,
    #[error("type mismatch in operation")]
    TypeMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JitType {
    Int,
    Float,
    Bool,
}

impl JitType {
    /// Returns the C type string for this JIT type
    pub fn to_c_type(&self) -> &'static str {
        match self {
            Self::Int => "int64_t",
            Self::Float => "double",
            Self::Bool => "int",
        }
    }
}

/// Compile Python bytecode to C source code.
///
/// Returns the generated C code as a string that can be compiled with
/// a standard C compiler (gcc, clang, etc.).
///
/// # Arguments
/// * `bytecode` - The Python bytecode to compile
/// * `args` - The types of the function arguments (must have type annotations)
/// * `ret` - The return type of the function (optional)
///
/// # Example output
/// ```c
/// #include <stdint.h>
///
/// int64_t add(int64_t a, int64_t b) {
///     int64_t tmp_0;
///     tmp_0 = a + b;
///     return tmp_0;
/// }
/// ```
pub fn compile<C: bytecode::Constant>(
    bytecode: &bytecode::CodeObject<C>,
    args: &[JitType],
    ret: Option<JitType>,
) -> Result<String, JitCompileError> {
    let mut generator = CCodeGenerator::new(bytecode, args, ret)?;
    generator.compile(bytecode)?;
    Ok(generator.into_code())
}
