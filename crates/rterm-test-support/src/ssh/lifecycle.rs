use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use tokio::task::{AbortHandle, JoinError, JoinHandle, JoinSet};

const ABORT_RESERVE: Duration = Duration::from_millis(50);

pub(super) type ReapFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
type TaskJoinFuture = Pin<Box<dyn Future<Output = Result<(), JoinError>> + Send + 'static>>;

enum ReapJob {
    Async(ReapFuture),
    Thread(thread::JoinHandle<()>),
}

static PROCESS_REAPER: OnceLock<Mutex<Option<tokio::sync::mpsc::UnboundedSender<ReapJob>>>> =
    OnceLock::new();
static PROCESS_REAPER_RETAINED_THREADS: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub(super) struct ShutdownDeadline {
    at: Instant,
    budget: Duration,
}

pub(super) enum ThreadJoinOutcome {
    Completed,
    Panicked,
    Deferred,
}

impl ShutdownDeadline {
    pub(super) fn after(budget: Duration) -> Self {
        Self {
            at: Instant::now()
                .checked_add(budget)
                .unwrap_or_else(Instant::now),
            budget,
        }
    }

    pub(super) fn at(self) -> Instant {
        self.at
    }

    pub(super) fn budget(self) -> Duration {
        self.budget
    }

    pub(super) fn remaining(self) -> Duration {
        self.at.saturating_duration_since(Instant::now())
    }

    pub(super) fn abort_at(self) -> Instant {
        self.at
            .checked_sub(ABORT_RESERVE)
            .unwrap_or_else(Instant::now)
            .max(Instant::now())
    }

    pub(super) async fn timeout<F>(
        self,
        future: F,
    ) -> Result<F::Output, tokio::time::error::Elapsed>
    where
        F: Future,
    {
        tokio::time::timeout_at(tokio::time::Instant::from_std(self.at), future).await
    }
}

pub(super) fn ensure_process_reaper() -> io::Result<()> {
    let state = PROCESS_REAPER.get_or_init(|| Mutex::new(None));
    let mut sender = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if sender.as_ref().is_some_and(|sender| !sender.is_closed()) {
        return Ok(());
    }
    let (new_sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    thread::Builder::new()
        .name("rterm-fixture-process-reaper".to_owned())
        .spawn(move || run_process_reaper(runtime, receiver))?;
    *sender = Some(new_sender);
    Ok(())
}

pub(super) fn defer_future(future: ReapFuture) -> Result<(), ReapFuture> {
    if ensure_process_reaper().is_err() {
        return Err(future);
    }
    let sender = PROCESS_REAPER
        .get()
        .expect("process reaper state initialized")
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("process reaper sender initialized");
    sender
        .send(ReapJob::Async(future))
        .map_err(|error| match error.0 {
            ReapJob::Async(future) => future,
            ReapJob::Thread(_) => unreachable!(),
        })
}

pub(super) fn defer_thread(worker: thread::JoinHandle<()>) {
    let worker = match try_defer_thread(worker) {
        Ok(()) => return,
        Err(worker) => worker,
    };
    let worker = match try_defer_thread(worker) {
        Ok(()) => return,
        Err(worker) => worker,
    };
    PROCESS_REAPER_RETAINED_THREADS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(worker);
}

pub(super) fn join_thread_until(
    worker: thread::JoinHandle<()>,
    deadline: ShutdownDeadline,
) -> ThreadJoinOutcome {
    while !worker.is_finished() && deadline.remaining() > Duration::ZERO {
        thread::park_timeout(deadline.remaining().min(Duration::from_millis(2)));
    }
    if worker.is_finished() {
        if worker.join().is_ok() {
            ThreadJoinOutcome::Completed
        } else {
            ThreadJoinOutcome::Panicked
        }
    } else {
        defer_thread(worker);
        ThreadJoinOutcome::Deferred
    }
}

fn try_defer_thread(worker: thread::JoinHandle<()>) -> Result<(), thread::JoinHandle<()>> {
    if ensure_process_reaper().is_err() {
        return Err(worker);
    }
    let sender = PROCESS_REAPER
        .get()
        .expect("process reaper state initialized")
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("process reaper sender initialized");
    sender
        .send(ReapJob::Thread(worker))
        .map_err(|error| match error.0 {
            ReapJob::Thread(worker) => worker,
            ReapJob::Async(_) => unreachable!(),
        })
}

fn run_process_reaper(
    runtime: tokio::runtime::Runtime,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<ReapJob>,
) {
    runtime.block_on(async move {
        let mut jobs = JoinSet::new();
        loop {
            tokio::select! {
                Some(job) = receiver.recv() => {
                    jobs.spawn(async move {
                        match job {
                            ReapJob::Async(future) => future.await,
                            ReapJob::Thread(worker) => reap_thread(worker).await,
                        }
                    });
                }
                _ = jobs.join_next(), if !jobs.is_empty() => {}
                else => break,
            }
        }
    });
    runtime.shutdown_background();
}

async fn reap_thread(worker: thread::JoinHandle<()>) {
    let mut worker = Some(worker);
    while !worker.as_ref().is_some_and(thread::JoinHandle::is_finished) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if let Some(worker) = worker.take() {
        let _ = worker.join();
    }
}

pub(super) struct OwnedTask {
    abort: AbortHandle,
    join: Option<TaskJoinFuture>,
}

impl OwnedTask {
    pub(super) fn from_join(join: JoinHandle<()>) -> Self {
        Self {
            abort: join.abort_handle(),
            join: Some(Box::pin(join)),
        }
    }

    pub(super) fn abort(&self) {
        self.abort.abort();
    }

    pub(super) async fn wait_until(&mut self, at: Instant) -> bool {
        let Some(join) = self.join.as_mut() else {
            return true;
        };
        if tokio::time::timeout_at(tokio::time::Instant::from_std(at), join)
            .await
            .is_ok()
        {
            self.join.take();
            true
        } else {
            false
        }
    }

    pub(super) fn defer(mut self) {
        if let Some(join) = self.join.take() {
            let future = Box::pin(async move {
                let _ = join.await;
            });
            if let Err(future) = defer_future(future) {
                // The singleton is initialized before fixture startup. If it is
                // unexpectedly unavailable, retain ownership by polling the join
                // to completion on a final dedicated process-reaper restart.
                let _ = defer_future(future);
            }
        }
    }
}

impl Drop for OwnedTask {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let future = Box::pin(async move {
                let _ = join.await;
            });
            let _ = defer_future(future);
        }
    }
}
