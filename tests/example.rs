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
