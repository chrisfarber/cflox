use std::ops::{Deref, DerefMut};

/// The heap allows you to allocate memory.
///
/// Because this is a toy interpreter, this heap is ... unsafe.
///
/// I'm going to totally violate Rust's ownership and mutability rules
/// behind this interface.
///
/// also the cake is a lie
#[derive(Debug)]
pub struct Heap {}

impl Heap {
    pub fn new() -> Self {
        Self {}
    }

    pub fn alloc<T>(&mut self, value: T) -> Gc<T> {
        let ptr = Box::into_raw(Box::new(value));
        Gc { ptr }
    }
}

#[derive(Debug)]
pub struct Gc<T> {
    ptr: *mut T,
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Gc<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i_can_leak_memory_yikes() {
        let mut heap = Heap::new();
        let mut wat = heap.alloc("hello?".to_owned());

        assert!(wat.is_ascii());

        wat.push('?');

        assert!(wat.eq("hello??"));

        let mut wat2 = wat;
        wat.push('!');
        wat2.push('~');
        assert!(wat2.eq(wat.deref()));
        assert!(wat2.eq("hello??!~"));
    }
}
