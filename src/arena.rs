/// Thin wrapper around bumpalo's Bump arena for order storage.
/// This will back the Vec<Order> inside each PriceLevel once integrated.
///
/// TODO: wire into PriceLevel so allocations come from here instead of the
/// global allocator.
pub struct Arena {
    inner: bumpalo::Bump,
}

impl Arena {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn with_capacity(_bytes: usize) -> Self {
        unimplemented!()
    }

    /// Allocate a T inside the arena. Lifetime tied to &self.
    pub fn alloc<T>(&self, _val: T) -> &mut T {
        unimplemented!()
    }

    /// Bytes currently allocated.
    pub fn allocated_bytes(&self) -> usize {
        unimplemented!()
    }

    /// Drop all allocations and reset the arena.
    pub fn reset(&mut self) {
        unimplemented!()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}
