#![cfg_attr(not(test), no_std)]
#![feature(ptr_metadata)]
#![feature(unsize)]

use core::{
    alloc::Layout,
    marker::{PhantomData, Unsize},
    mem::MaybeUninit,
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

pub struct Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    align_offset: usize,
    mem: M,
    phantom: PhantomData<T>,
}

impl<T, M> Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline(always)]
    pub fn new_in_buf<Value>(mem: M, value: Value) -> Self
    where
        Value: Unsize<T>,
    {
        let (meta, layout, offset) = meta_offset_layout(&value);
        // Check that the provided buffer has sufficient capacity to store the given value.
        assert!(layout.size() > 0);

        // Construct a box to move the specified memory into the necessary location.
        // Safety: This code relies on the fact that this method will be inlined.
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

impl<T, const N: usize> Box<T, [u8; N]>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
{
    #[inline]
    pub fn new<Value>(value: Value) -> Self
    where
        Value: Unsize<T>,
    {
        let mem: MaybeUninit<[u8; N]> = MaybeUninit::uninit();
        Self::new_in_buf(unsafe { mem.assume_init() }, value)
    }
}

impl<T, M> AsRef<T> for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.value_ptr() }
    }
}

impl<T, M> AsMut<T> for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
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

    #[inline]
    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T, M> DerefMut for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T, M> Drop for Box<T, M>
where
    T: ?Sized + Pointee<Metadata = DynMetadata<T>>,
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place::<T>(&mut **self);
        }
    }
}
