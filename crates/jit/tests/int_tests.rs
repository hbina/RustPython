/// Tests for integer operations - verify C code generation

#[test]
fn test_add() {
    let c_code = jit_function! { add => r##"
        def add(a: int, b: int) -> int:
            return a + b
    "## };

    // Verify function signature
    assert!(c_code.contains("int64_t add(int64_t a, int64_t b)"));
    // Verify includes
    assert!(c_code.contains("#include <stdint.h>"));
    // Verify addition operation
    assert!(c_code.contains("a + b"));
    // Verify return statement
    assert!(c_code.contains("return"));
}

#[test]
fn test_sub() {
    let c_code = jit_function! { sub => r##"
        def sub(a: int, b: int) -> int:
            return a - b
    "## };

    assert!(c_code.contains("int64_t sub(int64_t a, int64_t b)"));
    assert!(c_code.contains("a - b"));
}

#[test]
fn test_mul() {
    let c_code = jit_function! { mul => r##"
        def mul(a: int, b: int) -> int:
            return a * b
    "## };

    assert!(c_code.contains("int64_t mul(int64_t a, int64_t b)"));
    assert!(c_code.contains("a * b"));
}

#[test]
fn test_div() {
    let c_code = jit_function! { div => r##"
        def div(a: int, b: int) -> float:
            return a / b
    "## };

    assert!(c_code.contains("double div(int64_t a, int64_t b)"));
    // True division should cast to double
    assert!(c_code.contains("(double)"));
}

#[test]
fn test_floor_div() {
    let c_code = jit_function! { floor_div => r##"
        def floor_div(a: int, b: int) -> int:
            return a // b
    "## };

    assert!(c_code.contains("int64_t floor_div(int64_t a, int64_t b)"));
    assert!(c_code.contains("a / b"));
}

#[test]
fn test_exp() {
    let c_code = jit_function! { exp => r##"
        def exp(a: int, b: int) -> int:
            return a ** b
    "## };

    assert!(c_code.contains("int64_t exp(int64_t a, int64_t b)"));
    // Should use the ipow helper function
    assert!(c_code.contains("__jit_ipow"));
    // Helper function should be defined
    assert!(c_code.contains("static int64_t __jit_ipow"));
}

#[test]
fn test_mod() {
    let c_code = jit_function! { modulo => r##"
        def modulo(a: int, b: int) -> int:
            return a % b
    "## };

    assert!(c_code.contains("int64_t modulo(int64_t a, int64_t b)"));
    assert!(c_code.contains("a % b"));
}

#[test]
fn test_lshift() {
    let c_code = jit_function! { lshift => r##"
        def lshift(a: int, b: int) -> int:
            return a << b
    "## };

    assert!(c_code.contains("int64_t lshift(int64_t a, int64_t b)"));
    assert!(c_code.contains("a << b"));
}

#[test]
fn test_rshift() {
    let c_code = jit_function! { rshift => r##"
        def rshift(a: int, b: int) -> int:
            return a >> b
    "## };

    assert!(c_code.contains("int64_t rshift(int64_t a, int64_t b)"));
    assert!(c_code.contains("a >> b"));
}

#[test]
fn test_and() {
    let c_code = jit_function! { bitand => r##"
        def bitand(a: int, b: int) -> int:
            return a & b
    "## };

    assert!(c_code.contains("int64_t bitand(int64_t a, int64_t b)"));
    assert!(c_code.contains("a & b"));
}

#[test]
fn test_or() {
    let c_code = jit_function! { bitor => r##"
        def bitor(a: int, b: int) -> int:
            return a | b
    "## };

    assert!(c_code.contains("int64_t bitor(int64_t a, int64_t b)"));
    assert!(c_code.contains("a | b"));
}

#[test]
fn test_xor() {
    let c_code = jit_function! { bitxor => r##"
        def bitxor(a: int, b: int) -> int:
            return a ^ b
    "## };

    assert!(c_code.contains("int64_t bitxor(int64_t a, int64_t b)"));
    assert!(c_code.contains("a ^ b"));
}

#[test]
fn test_eq() {
    let c_code = jit_function! { eq => r##"
        def eq(a: int, b: int) -> int:
            if a == b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t eq(int64_t a, int64_t b)"));
    assert!(c_code.contains("=="));
    // Should have conditional jump
    assert!(c_code.contains("if") || c_code.contains("goto"));
}

#[test]
fn test_gt() {
    let c_code = jit_function! { gt => r##"
        def gt(a: int, b: int) -> int:
            if a > b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t gt(int64_t a, int64_t b)"));
    assert!(c_code.contains(">"));
}

#[test]
fn test_lt() {
    let c_code = jit_function! { lt => r##"
        def lt(a: int, b: int) -> int:
            if a < b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t lt(int64_t a, int64_t b)"));
    assert!(c_code.contains("<"));
}

#[test]
fn test_gte() {
    let c_code = jit_function! { gte => r##"
        def gte(a: int, b: int) -> int:
            if a >= b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t gte(int64_t a, int64_t b)"));
    assert!(c_code.contains(">="));
}

#[test]
fn test_lte() {
    let c_code = jit_function! { lte => r##"
        def lte(a: int, b: int) -> int:
            if a <= b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t lte(int64_t a, int64_t b)"));
    assert!(c_code.contains("<="));
}

#[test]
fn test_minus() {
    let c_code = jit_function! { minus => r##"
        def minus(a: int) -> int:
            return -a
    "## };

    assert!(c_code.contains("int64_t minus(int64_t a)"));
    assert!(c_code.contains("-a"));
}
