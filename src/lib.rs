#![cfg_attr(not(test), no_std)]
#![feature(ptr_metadata)]
#![feature(unsize)]

use core::{
    alloc::Layout,
    marker::{PhantomData, Unsize},
    ops::{Deref, DerefMut},
    ptr::{self, drop_in_place, metadata, DynMetadata, NonNull, Pointee},
};

#[cfg(test)]
mod tests;

//

pub struct Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    mem: M,
    phantom: PhantomData<T>,
}

impl<T, M> Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    pub fn new_in_buf<Value>(mut mem: M, value: Value) -> Self
    where
        Value: Unsize<T>,
    {
        let meta = metadata(&value as &T);

        let meta_layout = Layout::for_value(&meta);
        let value_layout = Layout::for_value(&value);
        let (layout, offset) = meta_layout.extend(value_layout).unwrap();
        // TODO
        assert!(layout.size() > 0);
        assert!(layout.size() <= mem.as_ref().len());

        unsafe {
            let ptr = NonNull::new(mem.as_mut().as_mut_ptr()).unwrap();
            // TODO
            ptr.cast::<DynMetadata<T>>().as_ptr().write(meta);
            // TODO
            ptr.cast::<u8>()
                .as_ptr()
                .add(offset)
                .cast::<Value>()
                .write(value);

            Self {
                mem,
                phantom: PhantomData,
            }
        }
    }

    fn meta(&self) -> DynMetadata<T> {
        unsafe { *self.mem.as_ref().as_ptr().cast() }
    }

    fn layout_meta(&self) -> (Layout, usize, DynMetadata<T>) {
        let meta = self.meta();
        let (layout, offset) = Layout::for_value(&meta).extend(meta.layout()).unwrap();
        (layout, offset, meta)
    }

    fn value_ptr(&self) -> *const T {
        let (_, offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self.mem.as_ref().as_ptr().add(offset).cast::<()>();
            ptr::from_raw_parts(ptr, meta)
        }
    }

    fn value_mut_ptr(&mut self) -> *mut T {
        let (_, offset, meta) = self.layout_meta();
        unsafe {
            let ptr = self.mem.as_mut().as_mut_ptr().add(offset).cast::<()>();
            ptr::from_raw_parts_mut(ptr, meta)
        }
    }
}

impl<T, const N: usize> Box<T, [u8; N]>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    pub fn new<Value>(value: Value) -> Self
    where
        Value: Unsize<T>,
    {
        // TODO Investigate if it is possible to safely use an uninitialized memory buffer here.
        Self::new_in_buf([0_u8; N], value)
    }
}

impl<T, M> AsRef<T> for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    fn as_ref(&self) -> &T {
        unsafe { &*self.value_ptr() }
    }
}

impl<T, M> AsMut<T> for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value_mut_ptr() }
    }
}

impl<T, M> Deref for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T, M> DerefMut for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T, M> Drop for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    fn drop(&mut self) {
        unsafe {
            drop_in_place::<T>(&mut **self);
        }
    }
}
