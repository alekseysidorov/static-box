#![cfg_attr(not(test), no_std)]
#![feature(ptr_metadata)]
#![feature(unsize)]

use core::{
    alloc::Layout,
    marker::{PhantomData, Unsize},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::{self, drop_in_place, metadata, DynMetadata, NonNull, Pointee},
};

#[cfg(test)]
mod tests;

// 

pub struct Box<T, const N: usize>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    mem: [u8; N],
    phantom: PhantomData<T>,
}

impl<T, const N: usize> Box<T, N>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    pub fn new<Value: Unsize<T>>(value: Value) -> Self {
        let meta = metadata(&value as &T);

        let meta_layout = Layout::for_value(&meta);
        let value_layout = Layout::for_value(&value);
        let (layout, offset) = meta_layout.extend(value_layout).unwrap();
        // TODO
        assert!(layout.size() > 0);
        assert!(layout.size() <= N);

        unsafe {
            let mut mem = MaybeUninit::uninit();
            let ptr = NonNull::new(mem.as_mut_ptr()).unwrap();
            // TODO
            ptr.cast::<DynMetadata<T>>().as_ptr().write(meta);
            // TODO
            ptr.cast::<u8>()
                .as_ptr()
                .add(offset)
                .cast::<Value>()
                .write(value);

            Self {
                mem: mem.assume_init(),
                phantom: PhantomData,
            }
        }
    }

    fn meta(&self) -> DynMetadata<T> {
        unsafe { *self.mem.as_ptr().cast() }
    }

    fn layout_meta(&self) -> (Layout, usize, DynMetadata<T>) {
        let meta = self.meta();
        let (layout, offset) = Layout::for_value(&meta).extend(meta.layout()).unwrap();
        (layout, offset, meta)
    }

    fn value_ptr(&self) -> *const T {
        let (_, offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self.mem.as_ptr().add(offset).cast::<()>();
            ptr::from_raw_parts(ptr, meta)
        }
    }

    fn value_mut_ptr(&mut self) -> *mut T {
        let (_, offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self.mem.as_mut_ptr().add(offset).cast::<()>();
            ptr::from_raw_parts_mut(ptr, meta)
        }
    }
}

impl<T, const N: usize> Deref for Box<T, N>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value_ptr() }
    }
}

impl<T, const N: usize> DerefMut for Box<T, N>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value_mut_ptr() }
    }
}

impl<T, const N: usize> Drop for Box<T, N>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    fn drop(&mut self) {
        unsafe {
            drop_in_place::<T>(&mut **self);
        }
    }
}
