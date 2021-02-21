//! This is a small library that enables section-style testing in Rust.
//! Section-style testing makes writing many similar test cases easy, natural,
//! and concise.
//!
//! Each top-level section test is run repeatedly, once for every unique
//! section inside the test. This is more expressive and natural than fixtures
//! because it lets you use local variables from parent scopes inside a section
//! and because you can nest sections to an arbitrary depth.
//!
//! Here's an example:
//!
//! ```rust,ignore
//! #[macro_use]
//! extern crate section_testing;
//!
//! enable_sections! {
//!   #[test]
//!   fn example_test() {
//!     let mut v: Vec<i32> = vec![];
//!
//!     fn check_123(v: &mut Vec<i32>) {
//!       assert_eq!(*v, vec![1, 2, 3]);
//!
//!       if section!("reverse") {
//!         v.reverse();
//!         assert_eq!(*v, vec![3, 2, 1]);
//!       }
//!
//!       if section!("pop+remove+insert+push") {
//!         let three = v.pop().unwrap();
//!         let one = v.remove(0);
//!         v.insert(0, three);
//!         v.push(one);
//!         assert_eq!(*v, vec![3, 2, 1]);
//!       }
//!     }
//!
//!     if section!("push") {
//!       v.push(1);
//!       v.push(2);
//!       v.push(3);
//!       check_123(&mut v);
//!     }
//!
//!     if section!("insert") {
//!       v.insert(0, 3);
//!       v.insert(0, 1);
//!       v.insert(1, 2);
//!       check_123(&mut v);
//!     }
//!   }
//! }
//! ```
//!
//! The `enable_sections!` macro modifies the test functions inside of it so
//! that they run repeatedly until all sections have been visited. The
//! `section!` macro returns a `bool` for whether or not that section should be
//! run this iteration. This example test will check the following combinations:
//!
//! ```text
//! push
//! push, reverse
//! push, pop+remove+insert+push
//! insert
//! insert, reverse
//! insert, pop+remove+insert+push
//! ```
//!
//! When a test fails, the enclosing sections will be printed to stderr. Here's
//! what happens if we comment out `v.push(one);` in the example above:
//!
//! ```text
//! running 1 test
//! thread 'example_test' panicked at 'assertion failed: `(left == right)`
//!   left: `[3, 2]`,
//!  right: `[3, 2, 1]`', src/main.rs:30:9
//! note: Run with `RUST_BACKTRACE=1` for a backtrace.
//! ---- the failure was inside these sections ----
//!   0) "push" at src/main.rs:34
//!   1) "pop+remove+insert+push" at src/main.rs:25
//! test example_test ... FAILED
//! ```
//!
//! Note that like all tests in Rust, a section-style test will stop on the
//! first failure. This means you will only be able to see the first combination
//! that failed instead of being able to see all failed combinations. The above
//! example would have also failed for the combination `insert,
//! pop+remove+insert+push` if the other combination hadn't failed first. This
//! is because Rust's built-in test runner has no API for adding new tests at
//! runtime.

use std::mem::swap;
use std::fmt::Write;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

thread_local! {
  static CURRENT_RUNNER: RefCell<Runner> = RefCell::new(Runner::new());
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Section {
  name: &'static str,
  file: &'static str,
  line: u32,
}

#[derive(Clone, Copy)]
struct Entry {
  should_enter: bool,
  index: usize,
}

struct Runner {
  is_running: bool,
  queue: VecDeque<HashMap<Section, Entry>>,
  current: HashMap<Section, Entry>,
  new: Vec<Section>,
}

impl Runner {
  fn new() -> Runner {
    Runner {
      is_running: false,
      queue: vec![HashMap::new()].into(),
      current: HashMap::new(),
      new: vec![],
    }
  }
}

pub struct DropHandler {
  pub is_top_level: bool,
  pub was_success: bool,
}

impl Drop for DropHandler {
  fn drop(&mut self) {
    if !self.is_top_level {
      return;
    }

    CURRENT_RUNNER.with(|r| {
      r.borrow_mut().is_running = false;

      // Did the test complete successfully?
      if self.was_success {
        let mut r = r.borrow_mut();
        let mut new = vec![];
        swap(&mut r.new, &mut new);

        // If so, add newly-discovered sections to the queue
        for section in &new {
          let mut path = r.current.clone();
          let count = r.current.values().filter(|x| x.should_enter).count();
          for s in &new {
            path.insert(*s, Entry {
              should_enter: s == section,
              index: count,
            });
          }
          r.queue.push_back(path);
        }
      }

      // Is the test in the middle of unwinding due to a panic?
      else {
        let mut current: Vec<_> = r.borrow().current.iter()
          .map(|(k, v)| (*k, *v))
          .filter(|(_, v)| v.should_enter)
          .collect();
        current.sort_unstable_by(|a, b| a.1.index.cmp(&b.1.index));

        // Write out the failure as a single buffer to avoid it interleaving with other output
        if !current.is_empty() {
          let mut buffer = "---- the failure was inside these sections ----\n".to_owned();
          for (i, (section, _)) in current.iter().enumerate() {
            writeln!(&mut buffer, "{: >3}) {:?} at {}:{}",
              i, section.name, section.file, section.line).unwrap();
          }
          eprint!("{}", buffer);
        }
      }
    });
  }
}

pub fn enable_sections_start() -> bool {
  CURRENT_RUNNER.with(|r| {
    if r.borrow().is_running {
      false
    } else {
      r.replace(Runner::new());
      true
    }
  })
}

pub fn enable_sections_step() -> bool {
  CURRENT_RUNNER.with(|r| {
    let mut r = r.borrow_mut();
    if let Some(current) = r.queue.pop_front() {
      r.current = current;
      r.new.clear();
      r.is_running = true;
      true
    } else {
      false
    }
  })
}

pub fn enter_section(name: &'static str, file: &'static str, line: u32) -> bool {
  CURRENT_RUNNER.with(|r| {
    let section = Section {name, file, line};
    let should_enter = r.borrow().current.get(&section).map(|x| x.should_enter);
    should_enter.unwrap_or_else(|| {
      r.borrow_mut().new.push(section);
      false
    })
  })
}

pub fn is_running() -> bool {
  CURRENT_RUNNER.with(|r| r.borrow().is_running)
}

#[macro_export]
macro_rules! enable_sections {
  (
    $(
      $(#[$($attrs:tt)*])*
      fn $name:ident() {
        $($arg:tt)*
      }
    )*
  ) => {
    $(
      $(#[$($attrs)*])*
      fn $name() {
        let is_top_level = $crate::enable_sections_start();
        loop {
          // Stop this run when the queue is empty
          if is_top_level && !$crate::enable_sections_step() {
            break;
          }

          // Run the function body
          let mut scope = $crate::DropHandler {is_top_level, was_success: false};
          $($arg)*
          scope.was_success = true;

          // Only run the function body once if we're not top-level
          if !is_top_level {
            break;
          }
        }
      }
    )*
  }
}

#[macro_export]
macro_rules! section {
  ($name:expr) => {{
    assert!($crate::is_running(), "\"section!(...)\" must be called from inside \"enable_sections! { ... }\"");
    $crate::enter_section($name, file!(), line!())
  }}
}
