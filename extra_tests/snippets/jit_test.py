def foo():
    a = 5
    return 10 + a


def bar():
    a = 1e6
    return a / 5.0


def baz(a: int, b: int):
    return a + b + 12


def return_tuple():
    return 10, 20


def check(f, *args):
    assert hasattr(f, "__jit__")
    prev = f(*args)
    f.__jit__()
    after = f(*args)
    assert prev == after


# check(foo)
# check(bar)
# check(baz, 17, 20)
# check(baz, 17, 22.5)
check(return_tuple)
