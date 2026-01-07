/// Tests for float operations - verify C code generation

#[test]
fn test_add() {
    let c_code = jit_function! { add => r##"
        def add(a: float, b: float) -> float:
            return a + b
    "## };

    assert!(c_code.contains("double add(double a, double b)"));
    assert!(c_code.contains("#include <stdint.h>"));
    assert!(c_code.contains("a + b"));
}

#[test]
fn test_add_with_integer() {
    let c_code = jit_function! { add => r##"
        def add(a: float, b: int) -> float:
            return a + b
    "## };

    assert!(c_code.contains("double add(double a, int64_t b)"));
    // Should cast int to double
    assert!(c_code.contains("(double)"));
}

#[test]
fn test_sub() {
    let c_code = jit_function! { sub => r##"
        def sub(a: float, b: float) -> float:
            return a - b
    "## };

    assert!(c_code.contains("double sub(double a, double b)"));
    assert!(c_code.contains("a - b"));
}

#[test]
fn test_sub_with_integer() {
    let c_code = jit_function! { sub => r##"
        def sub(a: int, b: float) -> float:
            return a - b
    "## };

    assert!(c_code.contains("double sub(int64_t a, double b)"));
    assert!(c_code.contains("(double)"));
}

#[test]
fn test_mul() {
    let c_code = jit_function! { mul => r##"
        def mul(a: float, b: float) -> float:
            return a * b
    "## };

    assert!(c_code.contains("double mul(double a, double b)"));
    assert!(c_code.contains("a * b"));
}

#[test]
fn test_mul_with_integer() {
    let c_code = jit_function! { mul => r##"
        def mul(a: float, b: int) -> float:
            return a * b
    "## };

    assert!(c_code.contains("double mul(double a, int64_t b)"));
    assert!(c_code.contains("(double)"));
}

#[test]
fn test_power() {
    let c_code = jit_function! { pow => r##"
        def pow(a: float, b: float) -> float:
            return a ** b
    "## };

    assert!(c_code.contains("double pow(double a, double b)"));
    // Should use math.h pow()
    assert!(c_code.contains("#include <math.h>"));
    assert!(c_code.contains("pow("));
}

#[test]
fn test_div() {
    let c_code = jit_function! { div => r##"
        def div(a: float, b: float) -> float:
            return a / b
    "## };

    assert!(c_code.contains("double div(double a, double b)"));
    assert!(c_code.contains("a / b"));
}

#[test]
fn test_div_with_integer() {
    let c_code = jit_function! { div => r##"
        def div(a: float, b: int) -> float:
            return a / b
    "## };

    assert!(c_code.contains("double div(double a, int64_t b)"));
    assert!(c_code.contains("(double)"));
}

#[test]
fn test_if_bool() {
    let c_code = jit_function! { if_bool => r##"
        def if_bool(a: float) -> int:
            if a:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t if_bool(double a)"));
    // Should have conditional check
    assert!(c_code.contains("if") || c_code.contains("goto"));
}

#[test]
fn test_float_eq() {
    let c_code = jit_function! { float_eq => r##"
        def float_eq(a: float, b: float) -> bool:
            return a == b
    "## };

    assert!(c_code.contains("int float_eq(double a, double b)"));
    assert!(c_code.contains("=="));
}

#[test]
fn test_float_ne() {
    let c_code = jit_function! { float_ne => r##"
        def float_ne(a: float, b: float) -> bool:
            return a != b
    "## };

    assert!(c_code.contains("int float_ne(double a, double b)"));
    assert!(c_code.contains("!="));
}

#[test]
fn test_float_gt() {
    let c_code = jit_function! { float_gt => r##"
        def float_gt(a: float, b: float) -> bool:
            return a > b
    "## };

    assert!(c_code.contains("int float_gt(double a, double b)"));
    assert!(c_code.contains(">"));
}

#[test]
fn test_float_gte() {
    let c_code = jit_function! { float_gte => r##"
        def float_gte(a: float, b: float) -> bool:
            return a >= b
    "## };

    assert!(c_code.contains("int float_gte(double a, double b)"));
    assert!(c_code.contains(">="));
}

#[test]
fn test_float_lt() {
    let c_code = jit_function! { float_lt => r##"
        def float_lt(a: float, b: float) -> bool:
            return a < b
    "## };

    assert!(c_code.contains("int float_lt(double a, double b)"));
    assert!(c_code.contains("<"));
}

#[test]
fn test_float_lte() {
    let c_code = jit_function! { float_lte => r##"
        def float_lte(a: float, b: float) -> bool:
            return a <= b
    "## };

    assert!(c_code.contains("int float_lte(double a, double b)"));
    assert!(c_code.contains("<="));
}
