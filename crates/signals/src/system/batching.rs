impl super::ReactiveSystem {
    /// Flush all queued effects
    pub fn flush(&mut self) {
        while self.notify_index < self.queued_length {
            let effect = self.queued[self.notify_index].unwrap();
            self.queued[self.notify_index] = None;
            self.notify_index += 1;
            self.run(effect);
        }
        self.notify_index = 0;
        self.queued_length = 0;
    }

    /// Start a new batch
    pub fn start_batch(&mut self) {
        self.batch_depth += 1;
    }

    /// End the current batch and flush if needed
    pub fn end_batch(&mut self) {
        self.batch_depth -= 1;
        if self.batch_depth == 0 {
            self.flush();
        }
    }

    /// Count the number of nodes and links
    pub fn count(&self) -> (usize, usize) {
        (self.nodes.len(), self.links.len())
    }
}
