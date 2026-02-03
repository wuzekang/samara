use crate::system::ReactiveSystemRef;

impl super::ReactiveSystem {
    /// Flush all queued effects
    pub fn flush(this: ReactiveSystemRef<Self>) {
        while this.borrow().notify_index < this.borrow().queued_length {
            let effect = this.borrow().queued[this.borrow().notify_index];
            this.borrow_mut().notify_index += 1;
            Self::run(this.clone(), effect);
        }
        this.borrow_mut().notify_index = 0;
        this.borrow_mut().queued_length = 0;
    }

    /// Start a new batch
    pub fn start_batch(&mut self) {
        self.batch_depth += 1;
    }

    /// End the current batch and flush if needed
    pub fn end_batch(this: ReactiveSystemRef<Self>) {
        this.borrow_mut().batch_depth -= 1;
        if this.borrow_mut().batch_depth == 0 {
            Self::flush(this);
        }
    }

    /// Count the number of nodes and links
    pub fn count(&self) -> (usize, usize) {
        (self.nodes.len(), self.links.len())
    }
}
