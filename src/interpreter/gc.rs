use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub type GcRef<'a, T> = Ref<'a, T>;
pub type GcRefMut<'a, T> = RefMut<'a, T>;

///
/// Let's pretend this is a garbage collected value.
///
/// It's actually reference counted because it's an Rc<RefCell<T>>.
/// But I'm ignoring that for the purposes of this toy interpreter.
///
/// I had originally hoped to make this type suitable for implementing a
/// real garbage collector behind in the future, but, I have abandoned
/// that. At least I learned a lot about rust along the way.
///
#[derive(Debug)]
pub struct Gc<T> {
    obj: Rc<RefCell<T>>,
}

impl<T> Gc<T> {
    pub fn new(obj: T) -> Self {
        Self {
            obj: Rc::new(RefCell::new(obj)),
        }
    }

    pub fn borrow(&self) -> GcRef<'_, T> {
        self.obj.borrow()
    }

    pub fn borrow_mut(&self) -> GcRefMut<'_, T> {
        self.obj.borrow_mut()
    }
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self {
            obj: Rc::clone(&self.obj),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_this_work() {
        let a = Gc::new(4.0);
        let _v = a.obj.borrow();
        drop(_v);
        let mut d = a.obj.borrow_mut();
        *d = 32.0;
        drop(d);

        assert_eq!(*a.obj.borrow(), 32.0);
    }
}
