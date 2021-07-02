#![cfg_attr(not(test), no_std)]
#![feature(ptr_metadata)]
#![feature(unsize)]
#![feature(const_pin)]
// #![deny(missing_docs)]

//! # Overview
//!
//! This crate allows saving DST objects in the provided buffer.
//! In this way, it allows users to create global dynamic objects on a `no_std`
//! environment without a global allocator.
//!
//! Imagine that you want to create a generic embedded logger which can be used for
//! any board regardless of its hardware details. But you cannot just declare
//! a static variable that implements some trait because the compiler doesn't know how
//! much memory should be used to allocate it. In such cases, you have to use trait objects
//! to erase the origin type. You might use the [`alloc::boxed::Box`](https://doc.rust-lang.org/stable/alloc/boxed/struct.Box.html)
//! to do this thing, but it depends on the global allocator, which you also should provide,
//! and there are a lot of caveats not to use heap on the embedded devices.
//!
//! Instead of using a global allocator, you can use this crate to store dynamic objects
//! in the static memory.
//!
//! # Examples
//!
//! ```
//! use static_box::Box;
//!
//! struct Uart1Rx {
//!     // Implementation details...
//! }
//!
//! # trait SerialWrite {
//! #    fn write(&mut self, byte: u8);
//! #    fn write_str(&mut self, _s: &str) {}
//! # }
//! #
//! impl SerialWrite for Uart1Rx {
//!     fn write(&mut self, _byte: u8) {
//!         // Implementation details
//!     }
//! }
//!
//! let rx = Uart1Rx { /* ... */ };
//! let mut mem = [0_u8; 32];
//! let mut writer = Box::<dyn SerialWrite>::new(&mut mem, rx);
//! writer.write_str("Hello world!");
//! ```
//!
//! A more complex example demonstrating the usage of an external buffer.
//! ```
//! use core::fmt::Display;
//! use static_box::Box;
//!
//! let mut mem = [0_u8; 64];
//!
//! let value = 42_u64;
//! // Calculate the amount of memory needed to store this object.
//! let total_len = {
//!     let layout = Box::<dyn Display>::layout_of_dyn(&value);
//!     let align_offset = mem.as_ptr().align_offset(layout.align());
//!     layout.size() + align_offset
//! };
//! let (head, _tail) = mem.split_at_mut(total_len);
//!
//! let val: Box<dyn Display> = Box::new(head, value);
//! assert_eq!(val.to_string(), "42");
//! ```
//!
//! # Limitations
//!
//! At the moment this crate can only store dynamic objects, but it's hard to imagine
//! use cases where there is a need to store sized objects in this box.
//!
//! # Minimum Supported `rustc` Version
//!
//! This crate uses the following unstable features:
//!
//! - [`ptr_metadata`](https://doc.rust-lang.org/unstable-book/library-features/ptr-metadata.html)
//! - [`unsize`](https://doc.rust-lang.org/unstable-book/library-features/unsize.html)
//!
//! In other words, the crate's supported **nightly** `rustc` version is `1.53.0`, but there
//! is no guarantee that this code will work fine on the newest versions.
//!
//! # Implementation details
//!
//! **This crate uses unsafe code!**
//!
//! This crate inspired by the [`thin_box`](https://github.com/rust-lang/rust/blob/5ade3fe32c8a742504aaddcbe0d6e498f8eae11d/library/core/tests/ptr.rs#L561)
//! example in the `rustc` tests repository.
//!
//! This implementation relies on that the type `V` can be coerced into the unsized `dyn T`,
//! see the [`Unsize`](https://doc.rust-lang.org/core/marker/trait.Unsize.html) trait documentation.
//! From the coerced `dyn T` it gets a pointer to metadata, which contains all the necessary
//! information to manipulate the concrete type stored inside a trait object. After that it copies
//! metadata and the origin value into the provided buffer.
//!
//! Thus, to get the pointer to the `dyn T`, you have to read the metadata given the memory alignment,
//! use them to calculate the memory layout of the object, and only after that collect this
//! all into the pointer. So, keep in mind that getting a reference to the stored `dyn T` could be too
//! expensive in some cases.
//!

use core::{
    alloc::Layout,
    marker::{PhantomData, Unsize},
    ops::{Deref, DerefMut},
    ptr::{self, DynMetadata, NonNull, Pointee},
};

#[cfg(test)]
mod tests;

#[inline]
fn meta_offset_layout<T, Value>(value: &Value) -> (DynMetadata<T>, Layout, usize)
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    Value: Unsize<T> + ?Sized,
{
    // Get dynamic metadata for the given value.
    let meta = ptr::metadata(value as &T);
    // Compute memory layout to store the value + its metadata.
    let meta_layout = Layout::for_value(&meta);
    let value_layout = Layout::for_value(value);
    let (layout, offset) = meta_layout.extend(value_layout).unwrap();
    (meta, layout, offset)
}

/// A box that uses the provided memory to store dynamic objects.
pub struct Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    align_offset: usize,
    mem: &'m mut [u8],
    phantom: PhantomData<T>,
}

impl<'m, T> Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    /// Places a `value` into the specified `mem` buffer. The user should provide enough memory
    /// to store the value with its metadata considering alignment requirements.
    ///
    /// # Panics
    ///
    /// - If the provided buffer is insufficient to store the value.
    pub fn new<Value>(mem: &'m mut [u8], value: Value) -> Self
    where
        Value: Unsize<T>,
    {
        let (meta, layout, offset) = meta_offset_layout(&value);
        assert!(layout.size() > 0, "Unsupported value layot");

        // Construct a box to move the specified memory into the necessary location.
        // SAFETY: This code relies on the fact that this method will be inlined.
        let mut new_box = Self {
            align_offset: 0,
            mem,
            phantom: PhantomData,
        };

        let raw_ptr = new_box.mem.as_mut().as_mut_ptr();
        // Compute the offset that needs to be applied to the pointer in order to make
        // it aligned correctly.
        new_box.align_offset = raw_ptr.align_offset(layout.align());

        let total_len = new_box.align_offset + layout.size();
        let buf_len = new_box.mem.as_ref().len();
        // Check that the provided buffer has sufficient capacity to store the given value.
        if total_len > buf_len {
            // At the moment we cannot rely on the regular drop implementation because
            // the box is in an inconsistent state.
            core::mem::forget(new_box);
            panic!(
                "Not enough memory to store the specified value (got: {}, needed: {})",
                buf_len, total_len,
            );
        }

        unsafe {
            let ptr = NonNull::new(raw_ptr.add(new_box.align_offset)).unwrap();
            // Store dynamic metadata at the beginning of the given memory buffer.
            ptr.cast::<DynMetadata<T>>().as_ptr().write(meta);
            // Store the value in the remainder of the memory buffer.
            ptr.cast::<u8>()
                .as_ptr()
                .add(offset)
                .cast::<Value>()
                .write(value);

            new_box
        }
    }

    /// Calculates layout describing a record that could be used
    /// to allocate backing structure for `Value`.
    #[inline]
    pub fn layout_of_dyn<Value>(value: &Value) -> Layout
    where
        Value: Unsize<T> + ?Sized,
    {
        meta_offset_layout::<T, Value>(value).1
    }

    #[inline]
    fn meta(&self) -> DynMetadata<T> {
        unsafe { *self.mem.as_ref().as_ptr().add(self.align_offset).cast() }
    }

    #[inline]
    fn layout_meta(&self) -> (Layout, usize, DynMetadata<T>) {
        let meta = self.meta();
        let (layout, offset) = Layout::for_value(&meta).extend(meta.layout()).unwrap();
        (layout, offset, meta)
    }

    #[inline]
    fn value_ptr(&self) -> *const T {
        let (_, value_offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self
                .mem
                .as_ref()
                .as_ptr()
                .add(self.align_offset)
                .add(value_offset)
                .cast::<()>();
            ptr::from_raw_parts(ptr, meta)
        }
    }

    #[inline]
    fn value_mut_ptr(&mut self) -> *mut T {
        let (_, value_offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self
                .mem
                .as_mut()
                .as_mut_ptr()
                .add(self.align_offset)
                .add(value_offset)
                .cast::<()>();
            ptr::from_raw_parts_mut(ptr, meta)
        }
    }
}

impl<'m, T> AsRef<T> for Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.value_ptr() }
    }
}

impl<'m, T> AsMut<T> for Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value_mut_ptr() }
    }
}

impl<'m, T> Deref for Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<'m, T> DerefMut for Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<'m, T> Drop for Box<'m, T>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place::<T>(&mut **self);
        }
    }
}
