/// Tests for bool operations - verify C code generation

#[test]
fn test_return() {
    let c_code = jit_function! { return_ => r##"
        def return_(a: bool) -> bool:
            return a
    "## };

    assert!(c_code.contains("int return_(int a)"));
    assert!(c_code.contains("return"));
}

#[test]
fn test_const_true() {
    let c_code = jit_function! { const_true => r##"
        def const_true(a: int) -> bool:
            return True
    "## };

    assert!(c_code.contains("int const_true(int64_t a)"));
    assert!(c_code.contains("return 1"));
}

#[test]
fn test_const_false() {
    let c_code = jit_function! { const_false => r##"
        def const_false(a: int) -> bool:
            return False
    "## };

    assert!(c_code.contains("int const_false(int64_t a)"));
    assert!(c_code.contains("return 0"));
}

#[test]
fn test_not() {
    let c_code = jit_function! { not_ => r##"
        def not_(a: bool) -> bool:
            return not a
    "## };

    assert!(c_code.contains("int not_(int a)"));
    assert!(c_code.contains("!"));
}

#[test]
fn test_if_not() {
    let c_code = jit_function! { if_not => r##"
        def if_not(a: bool) -> int:
            if not a:
                return 0
            else:
                return 1
            return -1
    "## };

    assert!(c_code.contains("int64_t if_not(int a)"));
    // Should have conditional logic
    assert!(c_code.contains("if") || c_code.contains("goto"));
}

#[test]
fn test_eq() {
    let c_code = jit_function! { eq => r##"
        def eq(a: bool, b: bool) -> int:
            if a == b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t eq(int a, int b)"));
    assert!(c_code.contains("=="));
}

#[test]
fn test_gt() {
    let c_code = jit_function! { gt => r##"
        def gt(a: bool, b: bool) -> int:
            if a > b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t gt(int a, int b)"));
    assert!(c_code.contains(">"));
}

#[test]
fn test_lt() {
    let c_code = jit_function! { lt => r##"
        def lt(a: bool, b: bool) -> int:
            if a < b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t lt(int a, int b)"));
    assert!(c_code.contains("<"));
}

#[test]
fn test_gte() {
    let c_code = jit_function! { gte => r##"
        def gte(a: bool, b: bool) -> int:
            if a >= b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t gte(int a, int b)"));
    assert!(c_code.contains(">="));
}

#[test]
fn test_lte() {
    let c_code = jit_function! { lte => r##"
        def lte(a: bool, b: bool) -> int:
            if a <= b:
                return 1
            return 0
    "## };

    assert!(c_code.contains("int64_t lte(int a, int b)"));
    assert!(c_code.contains("<="));
}
