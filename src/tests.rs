use std::{
    fmt::{Debug, Display},
    sync::mpsc,
};

use crate::Box;

#[test]
fn test_box_trait_object() {
    let four = Box::<dyn Display, [u8; 32]>::new(4);
    assert_eq!(four.to_string(), "4");
    drop(four);

    let seven = Box::<dyn Display, [u8; 32]>::new(7);
    assert_eq!(seven.to_string(), "7");
}

#[test]
fn test_box_move() {
    fn move_me(b: Box<dyn Display, [u8; 32]>) {
        assert_eq!(b.to_string(), "4");
    }

    let b = Box::<dyn Display, [u8; 32]>::new(4);
    move_me(b);
}

#[test]
#[should_panic]
fn test_box_insufficient_memory() {
    let _four = Box::<dyn Display, [u8; 2]>::new(4);
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
    let b = Box::<dyn Debug, [u8; 32]>::new(Foo { tx });
    drop(b);

    assert_eq!(rx.recv().unwrap(), 42);
}

#[test]
fn test_box_in_provided_memory() {
    let mut mem = [0_u8; 32];

    let val: Box<dyn Display, _> = Box::new_in_buf(&mut mem, 42);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_layout_of_dyn() {
    let value = 42;

    let layout = Box::<dyn Display, &mut [u8]>::layout_of_dyn(&value);
    let mut mem = vec![0_u8; layout.size()];

    let val: Box<dyn Display, _> = Box::new_in_buf(&mut mem, value);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_box_dyn_fn() {
    let a = 42;
    let closure = move || a;
    let b = Box::<dyn Fn() -> i32, [u8; 32]>::new(closure);
    assert_eq!(b(), 42);
}

#[test]
fn test_box_nested_dyn_fn() {
    let closure = move |d: &dyn Fn(i32) -> String| d(42);

    let b = Box::<dyn Fn(&dyn Fn(i32) -> String) -> String, [u8; 32]>::new(closure);
    assert_eq!(b(&|a| a.to_string()), "42");
}
