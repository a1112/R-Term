use std::thread::JoinHandle;

/// Join handles transferred out of the live pane map after a close deadline.
#[derive(Debug, Default)]
pub(crate) struct WorkerReaper {
    joins: Vec<JoinHandle<()>>,
}

impl WorkerReaper {
    pub(crate) fn handoff(&mut self, join: JoinHandle<()>) {
        self.joins.push(join);
    }

    pub(crate) fn reap_finished(&mut self) {
        let mut pending = Vec::with_capacity(self.joins.len());
        for join in self.joins.drain(..) {
            if join.is_finished() {
                let _ = join.join();
            } else {
                pending.push(join);
            }
        }
        self.joins = pending;
    }

    pub(crate) fn join_all(&mut self) {
        for join in self.joins.drain(..) {
            let _ = join.join();
        }
    }
}
