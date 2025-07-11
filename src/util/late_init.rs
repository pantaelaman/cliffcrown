use std::{
  cell::SyncUnsafeCell,
  mem::MaybeUninit,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

pub struct LateInitialiser<T> {
  initialised: Arc<AtomicBool>,
  value_ref: Arc<SyncUnsafeCell<MaybeUninit<T>>>,
}

impl<T> LateInitialiser<T> {
  pub fn initialise(self, value: T) {
    unsafe { &mut *self.value_ref.get() }.write(value);
    self.initialised.store(true, Ordering::Release);
  }
}

pub struct LateInitialisee<T> {
  initialised: Arc<AtomicBool>,
  value_ref: Arc<SyncUnsafeCell<MaybeUninit<T>>>,
}

impl<T> LateInitialisee<T> {
  pub fn get(&self) -> Option<&T> {
    if !self.initialised.load(Ordering::Acquire) {
      return None;
    }

    Some(unsafe { (&*self.value_ref.get()).assume_init_ref() })
  }

  pub fn get_mut(&mut self) -> Option<&mut T> {
    if !self.initialised.load(Ordering::Acquire) {
      return None;
    }

    Some(unsafe { (&mut *self.value_ref.get()).assume_init_mut() })
  }
}

impl<T> std::ops::Drop for LateInitialisee<T> {
  fn drop(&mut self) {
    if self.initialised.load(Ordering::Acquire) {
      unsafe {
        (&mut *self.value_ref.get()).assume_init_drop();
      }
    }
  }
}

pub fn late_initialise<T>() -> (LateInitialiser<T>, LateInitialisee<T>) {
  let initialised = Arc::new(AtomicBool::new(false));
  let value_ref = Arc::new(SyncUnsafeCell::new(MaybeUninit::uninit()));

  (
    LateInitialiser {
      initialised: initialised.clone(),
      value_ref: value_ref.clone(),
    },
    LateInitialisee {
      initialised,
      value_ref,
    },
  )
}
