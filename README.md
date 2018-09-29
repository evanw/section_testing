This is a small library that enables section-style testing in Rust.
Section-style testing makes writing many similar test cases easy, natural,
and concise.

Each top-level section test is run repeatedly, once for every unique
section inside the test. This is more expressive and natural than fixtures
because it lets you use local variables from parent scopes inside a section
and because you can nest sections to an arbitrary depth.

# Getting Started

This library is published at https://crates.io/crates/section_testing. Add the
following dependency to your `Cargo.toml`:

```
[dependencies]
section_testing = "0.0.4"
```

Read the example below to learn how to use it.

# Example

Here's an example:

```rust
#[macro_use]
extern crate section_testing;

enable_sections! {
  #[test]
  fn example_test() {
    let mut v: Vec<i32> = vec![];

    fn check_123(v: &mut Vec<i32>) {
      assert_eq!(*v, vec![1, 2, 3]);

      if section!("reverse") {
        v.reverse();
        assert_eq!(*v, vec![3, 2, 1]);
      }

      if section!("pop+remove+insert+push") {
        let three = v.pop().unwrap();
        let one = v.remove(0);
        v.insert(0, three);
        v.push(one);
        assert_eq!(*v, vec![3, 2, 1]);
      }
    }

    if section!("push") {
      v.push(1);
      v.push(2);
      v.push(3);
      check_123(&mut v);
    }

    if section!("insert") {
      v.insert(0, 3);
      v.insert(0, 1);
      v.insert(1, 2);
      check_123(&mut v);
    }
  }
}
```

The `enable_sections!` macro modifies the test functions inside of it so
that they run repeatedly until all sections have been visited. The
`section!` macro returns a `bool` for whether or not that section should be
run this iteration. This example test will check the following combinations:

```
push
push, reverse
push, pop+remove+insert+push
insert
insert, reverse
insert, pop+remove+insert+push
```

When a test fails, the enclosing sections will be printed to stderr. Here's
what happens if we comment out `v.push(one);` in the example above:

```
running 1 test
thread 'example_test' panicked at 'assertion failed: `(left == right)`
  left: `[3, 2]`,
 right: `[3, 2, 1]`', src/main.rs:30:9
note: Run with `RUST_BACKTRACE=1` for a backtrace.
---- the failure was inside these sections ----
  0) "push" at src/main.rs:34
  1) "pop+remove+insert+push" at src/main.rs:25
test example_test ... FAILED
```

Note that like all tests in Rust, a section-style test will stop on the
first failure. This means you will only be able to see the first combination
that failed instead of being able to see all failed combinations. The above
example would have also failed for the combination `insert,
pop+remove+insert+push` if the other combination hadn't failed first. This
is because Rust's built-in test runner has no API for adding new tests at
runtime.
