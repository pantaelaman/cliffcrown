use std::sync::atomic::{AtomicBool, Ordering};

pub struct ChangeDetector<T> {
  target: T,
  changed: AtomicBool,
}

impl<T> ChangeDetector<T> {
  pub fn new(target: T) -> Self {
    ChangeDetector {
      target,
      changed: AtomicBool::new(true),
    }
  }

  pub fn changed(&self) -> bool {
    self.changed.load(Ordering::Relaxed)
  }

  pub fn take_change(&self) -> bool {
    if self.changed() {
      self.changed.store(false, Ordering::Relaxed);
      true
    } else {
      false
    }
  }

  pub fn get_if_changed(&self) -> Option<&T> {
    self.take_change().then_some(&*self)
  }
}

impl<T> std::ops::Deref for ChangeDetector<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.target
  }
}

impl<T> std::ops::DerefMut for ChangeDetector<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.changed.store(true, Ordering::Relaxed);
    &mut self.target
  }
}
