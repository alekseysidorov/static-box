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
        assert_eq!(b.to_string(), "42");
    }

    fn move_from_box() -> Box<dyn Display, [u8; 32]> {
        Box::<dyn Display, [u8; 32]>::new(42)
    }

    let b = Box::<dyn Display, [u8; 32]>::new(42);
    move_me(b);

    let x = move_from_box();
    assert_eq!(x.to_string(), "42");
}

#[test]
#[should_panic(expected = "Not enough memory")]
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
fn test_layout_of_dyn_vec() {
    let value = 42_u64;

    let layout = Box::<dyn Display, &mut [u8]>::layout_of_dyn(&value);
    dbg!(&layout);
    let mut mem = vec![0_u8; layout.size()];

    let val: Box<dyn Display, _> = Box::new_in_buf(&mut mem, value);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_layout_of_dyn_split_at_mut() {
    let mut mem = [0_u8; 64];

    let value = 42_u64;

    let total_len = {
        let layout = Box::<dyn Display, &mut [u8]>::layout_of_dyn(&value);
        let align_offset = mem.as_ptr().align_offset(layout.align());
        layout.size() + align_offset
    };
    let (head, _tail) = mem.split_at_mut(total_len);

    let val: Box<dyn Display, _> = Box::new_in_buf(head, value);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_box_dyn_fn() {
    let a = 42;
    let closure = move || a;
    let b = Box::<dyn Fn() -> i32, [u8; 64]>::new(closure);
    assert_eq!(b(), 42);
}

#[test]
fn test_box_nested_dyn_fn() {
    let closure = move |d: &dyn Fn(i32) -> String| d(42);

    let b = Box::<dyn Fn(&dyn Fn(i32) -> String) -> String, [u8; 32]>::new(closure);
    assert_eq!(b(&|a| a.to_string()), "42");
}

#[test]
fn test_box_in_unaligned_memory() {
    let mut mem = [0_u8; 128];

    let val: Box<dyn Display, _> = Box::new_in_buf(&mut mem[3..], 42);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_box_in_static_mem() {
    static mut BOX: Option<Box<dyn Display, [u8; 32]>> = None;

    unsafe {
        BOX.replace(Box::<dyn Display, [u8; 32]>::new(42));
        assert_eq!(BOX.as_ref().unwrap().to_string(), "42");
    }
}
