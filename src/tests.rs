use std::{
    fmt::{Debug, Display},
    sync::mpsc,
};

use crate::Box;

#[test]
fn test_box_trait_object() {
    let four = Box::<dyn Display, 32>::new(4);
    assert_eq!(four.to_string(), "4");
    drop(four);

    let seven = Box::<dyn Display, 32>::new(7);
    assert_eq!(seven.to_string(), "7");
}

#[test]
fn test_box_move() {
    fn move_me(b: Box<dyn Display, 32>) {
        assert_eq!(b.to_string(), "4");
    }

    let b = Box::<dyn Display, 32>::new(4);
    move_me(b);
}

#[test]
#[should_panic]
fn test_box_insufficient_memory() {
    let _four = Box::<dyn Display, 2>::new(4);
}

#[test]
fn test_drop() {
    #[derive(Debug)]
    struct Foo {
        tx: mpsc::Sender<i32>,
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            self.tx.send(42).unwrap();
        }
    }

    let (tx, rx) = mpsc::channel();
    let b = Box::<dyn Debug, 32>::new(Foo { tx });
    drop(b);

    assert_eq!(rx.recv().unwrap(), 42);
}
