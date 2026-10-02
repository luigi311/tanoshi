//! Share one batch among queue viewers, and do no aggregation without viewers.
//! The coordinator protects the boundary between queue commits and their
//! versions. It never gates library, settings, or other ordinary reads.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{Stream, StreamExt};
use tokio::sync::{OwnedMutexGuard, broadcast, watch};

use super::DownloadRepositoryImpl;
use crate::domain::{
    entities::download::DownloadQueueUpdate,
    repositories::download::{DownloadRepository, DownloadRepositoryError},
};

const BATCH_INTERVAL: Duration = Duration::from_secs(1);
const BUFFERED_BATCHES: usize = 32;

#[derive(Clone, Default)]
pub(super) struct QueueUpdates {
    state: Arc<Mutex<State>>,
    boundary: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Default)]
struct State {
    session: Option<Arc<Session>>,
    version: i64,
    published_version: i64,
    dirty: BTreeSet<i64>,
}

struct Session {
    sender: broadcast::Sender<Arc<DownloadQueueUpdate>>,
    stop: watch::Sender<bool>,
}

pub(super) struct QueueMutation {
    updates: QueueUpdates,
    _boundary: OwnedMutexGuard<()>,
    finished: bool,
}

impl QueueMutation {
    pub(super) fn complete(mut self, chapters: impl IntoIterator<Item = i64>) {
        self.updates.changed(chapters);
        self.finished = true;
    }

    pub(super) fn unchanged(mut self) {
        self.finished = true;
    }
}

impl Drop for QueueMutation {
    fn drop(&mut self) {
        // Cancellation during a SQL await may leave its outcome unknown.
        // Existing viewers must obtain a new snapshot rather than silently
        // retaining data whose publication was interrupted.
        if !self.finished {
            self.updates.resync();
        }
    }
}

struct Subscriber {
    updates: QueueUpdates,
    session: Arc<Session>,
    receiver: Option<broadcast::Receiver<Arc<DownloadQueueUpdate>>>,
}

impl Subscriber {
    fn into_stream(self) -> impl Stream<Item = DownloadQueueUpdate> + Send {
        futures::stream::unfold(self, |mut subscriber| async move {
            match subscriber.receiver.as_mut().unwrap().recv().await {
                Ok(update) => Some(((*update).clone(), subscriber)),
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!(target: "tanoshi::download_queue", "queue subscriber lagged: skipped_batches={skipped}; requesting a fresh snapshot");
                    Some((DownloadQueueUpdate::resync_required(), subscriber))
                }
                Err(broadcast::error::RecvError::Closed) => None,
            }
        })
    }
}

impl Drop for Subscriber {
    fn drop(&mut self) {
        let mut state = self.updates.state.lock().unwrap();
        self.receiver.take();
        let remaining = self.session.sender.receiver_count();
        debug!(target: "tanoshi::download_queue", "queue subscriber left: remaining={remaining}");
        if state
            .session
            .as_ref()
            .is_some_and(|session| Arc::ptr_eq(session, &self.session))
            && remaining == 0
        {
            debug!(target: "tanoshi::download_queue", "queue batching stopped: version={} -> 0, discarded_chapters={}", state.version, state.dirty.len());
            self.session.stop.send_replace(true);
            // A later viewer gets a separate channel. An old flush can never
            // publish into that channel, even while its SQL await is finishing.
            *state = State::default();
        }
    }
}

impl QueueUpdates {
    pub(super) async fn mutation(&self) -> QueueMutation {
        QueueMutation {
            updates: self.clone(),
            _boundary: self.boundary.clone().lock_owned().await,
            finished: false,
        }
    }

    fn changed(&self, chapters: impl IntoIterator<Item = i64>) {
        let mut state = self.state.lock().unwrap();
        if state.session.is_some() {
            state.version += 1;
            state.dirty.extend(chapters);
        }
    }

    fn resync(&self) {
        if let Some(session) = &self.state.lock().unwrap().session {
            debug!(target: "tanoshi::download_queue", "queue mutation interrupted: requesting fresh snapshots for {} subscribers", session.sender.receiver_count());
            let _ = session
                .sender
                .send(Arc::new(DownloadQueueUpdate::resync_required()));
        }
    }

    fn subscribe(&self) -> (Subscriber, bool) {
        let mut state = self.state.lock().unwrap();
        let start = state.session.is_none();
        let session = state
            .session
            .get_or_insert_with(|| {
                let (sender, receiver) = broadcast::channel(BUFFERED_BATCHES);
                drop(receiver);
                let (stop, _) = watch::channel(false);
                Arc::new(Session { sender, stop })
            })
            .clone();
        let receiver = Some(session.sender.subscribe());
        debug!(target: "tanoshi::download_queue", "queue subscriber joined: subscribers={} version={} batching_started={start}", session.sender.receiver_count(), state.version);
        (
            Subscriber {
                updates: self.clone(),
                session,
                receiver,
            },
            start,
        )
    }
}

impl DownloadRepositoryImpl {
    pub async fn subscribe_download_queue(
        &self,
    ) -> Result<impl Stream<Item = DownloadQueueUpdate> + Send + use<>, DownloadRepositoryError>
    {
        // Register first: changes during snapshot loading are already buffered.
        let (subscriber, start) = self.updates.subscribe();
        if start {
            let repo = self.clone();
            let session = subscriber.session.clone();
            tokio::spawn(async move {
                repo.run_queue_batches(session).await;
            });
        }

        let boundary = self.updates.boundary.lock().await;
        self.finish_interrupted_queue_write().await?;
        let updates = self.get_download_queue(&[]).await?;
        let version = self.updates.state.lock().unwrap().version;
        drop(boundary);
        let snapshot = DownloadQueueUpdate {
            snapshot: true,
            from_version: version,
            version,
            updates,
            removed_ids: vec![],
            resync_required: false,
        };
        Ok(futures::stream::once(async move { snapshot }).chain(subscriber.into_stream()))
    }

    async fn finish_interrupted_queue_write(&self) -> Result<(), DownloadRepositoryError> {
        // The pool's default acquire health check pings SQLite's worker,
        // finishing work whose caller was cancelled. No reader slot is held
        // while waiting, and the writer is released before the snapshot read.
        drop(self.pool.write().acquire().await?);
        Ok(())
    }

    async fn run_queue_batches(&self, session: Arc<Session>) {
        let mut stop = session.stop.subscribe();
        let mut interval =
            tokio::time::interval_at(tokio::time::Instant::now() + BATCH_INTERVAL, BATCH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if *stop.borrow() {
                break;
            }
            tokio::select! {
                _ = stop.changed() => break,
                _ = interval.tick() => {
                    if let Err(error) = self.flush_queue_batch(&session).await {
                        error!("queue subscription update failed: {error}");
                        let _ = session.sender.send(Arc::new(DownloadQueueUpdate::resync_required()));
                    }
                }
            }
        }
    }

    async fn flush_queue_batch(
        &self,
        session: &Arc<Session>,
    ) -> Result<(), DownloadRepositoryError> {
        let boundary = self.updates.boundary.lock().await;
        let pending = {
            let state = self.updates.state.lock().unwrap();
            if !state
                .session
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, session))
                || state.version == state.published_version
            {
                return Ok(());
            }
            (
                state.published_version,
                state.version,
                state.dirty.iter().copied().collect::<Vec<_>>(),
            )
        };
        let (from_version, version, dirty) = pending;
        self.finish_interrupted_queue_write().await?;
        // An empty chapter set can represent a pause/resume notification; it
        // must not call the full-queue query whose empty filter means "all".
        let updates = if dirty.is_empty() {
            vec![]
        } else {
            self.get_download_queue(&dirty).await?
        };
        let present: BTreeSet<_> = updates.iter().map(|entry| entry.chapter_id).collect();
        let removed_ids = dirty
            .into_iter()
            .filter(|id| !present.contains(id))
            .collect();
        {
            let mut state = self.updates.state.lock().unwrap();
            if !state
                .session
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, session))
            {
                return Ok(());
            }
            state.published_version = version;
            state.dirty.clear();
        }
        let _ = session.sender.send(Arc::new(DownloadQueueUpdate {
            snapshot: false,
            from_version,
            version,
            updates,
            removed_ids,
            resync_required: false,
        }));
        drop(boundary);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn last_subscriber_resets_versions_and_pending_changes() {
        let updates = QueueUpdates::default();
        updates.mutation().await.complete([1]);
        assert_eq!(updates.state.lock().unwrap().version, 0);
        assert!(updates.state.lock().unwrap().dirty.is_empty());

        let (first, start) = updates.subscribe();
        assert!(start);
        let (second, start) = updates.subscribe();
        assert!(!start);
        updates.mutation().await.complete([1, 1, 2]);
        updates.mutation().await.complete([2]);
        drop(first);
        {
            let state = updates.state.lock().unwrap();
            assert_eq!(state.version, 2);
            assert_eq!(state.dirty, BTreeSet::from([1, 2]));
        }
        let old_session = second.session.clone();
        drop(second);
        assert!(*old_session.stop.borrow());
        {
            let state = updates.state.lock().unwrap();
            assert!(state.session.is_none());
            assert_eq!(state.version, 0);
            assert_eq!(state.published_version, 0);
            assert!(state.dirty.is_empty());
        }

        updates.mutation().await.complete([3]);
        let (mut next, start) = updates.subscribe();
        assert!(start);
        assert!(!Arc::ptr_eq(&old_session, &next.session));
        // A previous batch finishing after a new viewer arrives cannot leak
        // into the new subscription even though both start at version zero.
        let _ = old_session
            .sender
            .send(Arc::new(DownloadQueueUpdate::resync_required()));
        assert!(matches!(
            next.receiver.as_mut().unwrap().try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        assert_eq!(updates.state.lock().unwrap().version, 0);
    }

    #[tokio::test]
    async fn slow_subscriber_is_told_to_resync_after_buffer_overflow() {
        let updates = QueueUpdates::default();
        let (subscriber, _) = updates.subscribe();
        for _ in 0..=BUFFERED_BATCHES {
            subscriber
                .session
                .sender
                .send(Arc::new(DownloadQueueUpdate {
                    resync_required: false,
                    ..DownloadQueueUpdate::resync_required()
                }))
                .unwrap();
        }
        let mut stream = Box::pin(subscriber.into_stream());
        assert!(stream.next().await.unwrap().resync_required);
    }

    #[tokio::test]
    async fn interrupted_mutation_requests_resync_but_no_op_does_not() {
        let updates = QueueUpdates::default();
        let (mut subscriber, _) = updates.subscribe();
        updates.mutation().await.unchanged();
        assert!(matches!(
            subscriber.receiver.as_mut().unwrap().try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        drop(updates.mutation().await);
        assert!(
            subscriber
                .receiver
                .as_mut()
                .unwrap()
                .try_recv()
                .unwrap()
                .resync_required
        );
        assert_eq!(updates.state.lock().unwrap().version, 0);
    }
}
