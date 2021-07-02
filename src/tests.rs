use std::{
    fmt::{Debug, Display},
    sync::mpsc,
};

use crate::Box;

#[test]
fn test_box_trait_object() {
    let mut mem = [0; 32];
    let four = Box::<dyn Display>::new(&mut mem, 4);
    assert_eq!(four.to_string(), "4");
    drop(four);

    let seven = Box::<dyn Display>::new(&mut mem, 7);
    assert_eq!(seven.to_string(), "7");
}

#[test]
fn test_box_move() {
    fn move_me(b: Box<dyn Display>) {
        assert_eq!(b.to_string(), "42");
    }

    fn move_from_box() -> Box<'static, dyn Display> {
        static mut STATIC_MEM: [u8; 32] = [0; 32];

        Box::<dyn Display>::new(unsafe { STATIC_MEM.as_mut() }, 42)
    }

    struct MyStruct<'a> {
        display: Box<'a, dyn Display>,
    }

    let mut mem = [0; 32];
    let b = Box::<dyn Display>::new(&mut mem, 42);
    move_me(b);

    let x = move_from_box();
    assert_eq!(x.to_string(), "42");

    let my_struct = MyStruct { display: x };
    assert_eq!(my_struct.display.to_string(), "42");
}

#[test]
#[should_panic(expected = "Not enough memory")]
fn test_box_insufficient_memory() {
    let mut mem = [0; 2];
    let _four = Box::<dyn Display>::new(&mut mem, 4);
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
    let mut mem = [0; 32];
    let b = Box::<dyn Debug>::new(&mut mem, Foo { tx });
    drop(b);

    assert_eq!(rx.recv().unwrap(), 42);
}

#[test]
fn test_layout_of_dyn_vec() {
    let value = 42_u64;

    let layout = Box::<dyn Display>::layout_of_dyn(&value);
    let mut mem = vec![0_u8; layout.size() + layout.align()];

    let val: Box<dyn Display> = Box::new(&mut mem, value);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_layout_of_dyn_split_at_mut() {
    let mut mem = [0_u8; 64];

    let value = 42_u64;

    let total_len = {
        let layout = Box::<dyn Display>::layout_of_dyn(&value);
        let align_offset = mem.as_ptr().align_offset(layout.align());
        layout.size() + align_offset
    };
    let (head, _tail) = mem.split_at_mut(total_len);

    let val: Box<dyn Display> = Box::new(head, value);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_box_dyn_fn() {
    let a = 42;
    let closure = move || a;

    let mut mem = [0; 64];
    let b = Box::<dyn Fn() -> i32>::new(&mut mem, closure);
    assert_eq!(b(), 42);
}

#[test]
fn test_box_nested_dyn_fn() {
    let closure = move |d: &dyn Fn(i32) -> String| d(42);

    let mut mem = [0; 32];
    let b = Box::<dyn Fn(&dyn Fn(i32) -> String) -> String>::new(&mut mem, closure);
    assert_eq!(b(&|a| a.to_string()), "42");
}

#[test]
fn test_box_in_unaligned_memory() {
    let mut mem = [0_u8; 128];

    let val: Box<dyn Display> = Box::new(&mut mem[3..], 42);
    assert_eq!(val.to_string(), "42");
}

#[test]
fn test_box_in_static_mem() {
    static mut MEM: [u8; 32] = [0; 32];
    static mut BOX: Option<Box<dyn Display>> = None;

    unsafe {
        BOX.replace(Box::<dyn Display>::new(&mut MEM, 42));
        assert_eq!(BOX.as_ref().unwrap().to_string(), "42");
    }
}
