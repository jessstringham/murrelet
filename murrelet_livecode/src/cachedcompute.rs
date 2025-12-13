use std::cell::OnceCell;

#[derive(Clone, Debug)]
pub struct CachedCompute<T: Clone + std::fmt::Debug>(OnceCell<T>);

impl<T: Clone + std::fmt::Debug> CachedCompute<T> {
    pub fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn has_been_set(&self) -> bool {
        self.0.get().is_some()
    }
}
