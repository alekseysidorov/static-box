use std::fmt::Display;

use crate::Box;

#[test]
fn test_box_simple() {
    let four = Box::<dyn Display, 32>::new(4);
    assert_eq!(four.to_string(), "4");
    drop(four);

    let seven = Box::<dyn Display, 32>::new(7);
    assert_eq!(seven.to_string(), "7");
}

#[test]
fn test_box_move() {
    fn move_me(b: Box::<dyn Display, 32>) {
        assert_eq!(b.to_string(), "4");
    }

    let b = Box::<dyn Display, 32>::new(4);
    move_me(b);
}
