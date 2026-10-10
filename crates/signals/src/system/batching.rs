use crate::system::ReactiveSystemRef;

impl super::ReactiveSystem {
    /// Flush all queued effects
    pub fn flush(this: &ReactiveSystemRef<Self>) {
        loop {
            let effect = {
                let mut sys = this.borrow_mut();
                if sys.notify_index >= sys.queued_length {
                    break;
                }
                let effect = sys.queued[sys.notify_index];
                sys.notify_index += 1;
                effect
            };
            // run() executes the effect body (user code) — no borrow held.
            Self::run(this, effect);
        }
        let mut sys = this.borrow_mut();
        sys.notify_index = 0;
        sys.queued_length = 0;
    }

    /// Start a new batch
    pub fn start_batch(&mut self) {
        self.batch_depth += 1;
    }

    /// End the current batch and flush if needed
    pub fn end_batch(this: &ReactiveSystemRef<Self>) {
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
