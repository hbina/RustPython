/// Miscellaneous tests - verify C code generation

#[test]
fn test_function_signature() {
    let c_code = jit_function! { func => r##"
        def func(a: int, b: float) -> int:
            return 1
    "## };

    // Verify function signature with mixed types
    assert!(c_code.contains("int64_t func(int64_t a, double b)"));
    assert!(c_code.contains("#include <stdint.h>"));
}

#[test]
fn test_if_else() {
    let c_code = jit_function! { if_else => r##"
        def if_else(a: int) -> int:
            if a:
                return 42
            else:
                return 0
            return 0
    "## };

    assert!(c_code.contains("int64_t if_else(int64_t a)"));
    // Should have conditional logic and multiple returns
    assert!(c_code.contains("goto") || c_code.contains("if"));
    assert!(c_code.contains("return 42") || c_code.contains("42LL"));
    assert!(c_code.contains("return 0") || c_code.contains("0LL"));
}

#[test]
fn test_while_loop() {
    let c_code = jit_function! { while_loop => r##"
        def while_loop(a: int) -> int:
            b = 0
            while a > 0:
                b += 1
                a -= 1
            return b
    "## };

    assert!(c_code.contains("int64_t while_loop(int64_t a)"));
    // Should have labels for loop
    assert!(c_code.contains("label_"));
    // Should have comparison and goto
    assert!(c_code.contains(">"));
    assert!(c_code.contains("goto"));
}

#[test]
fn test_local_variables() {
    let c_code = jit_function! { local_vars => r##"
        def local_vars(a: int, b: int) -> int:
            c = a + b
            d = c * 2
            return d
    "## };

    assert!(c_code.contains("int64_t local_vars(int64_t a, int64_t b)"));
    // Should have local variable declarations
    assert!(c_code.contains("int64_t var_") || c_code.contains("int64_t tmp_"));
}

#[test]
fn test_constant_return() {
    let c_code = jit_function! { const_ret => r##"
        def const_ret(a: int) -> int:
            return 42
    "## };

    assert!(c_code.contains("int64_t const_ret(int64_t a)"));
    assert!(c_code.contains("42"));
}
