use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

use futures_signals::{signal::Mutable, signal_vec::MutableVec};

use super::DownloadQueue;

pub struct DownloadQueueRow {
    pub chapter_id: i64,
    pub data: Mutable<DownloadQueue>,
}

impl DownloadQueueRow {
    fn order(&self) -> (i64, i64, i64) {
        let data = self.data.lock_ref();
        (data.priority, data.date_added, data.chapter_id)
    }
}

pub struct DownloadQueueUpdate {
    pub snapshot: bool,
    pub from_version: i64,
    pub version: i64,
    pub updates: Vec<DownloadQueue>,
    pub removed_ids: Vec<i64>,
    pub resync_required: bool,
    pub download_status: bool,
}

#[derive(Default)]
pub struct DownloadQueueState {
    pub rows: MutableVec<Rc<DownloadQueueRow>>,
    by_id: RefCell<HashMap<i64, Rc<DownloadQueueRow>>>,
    version: Cell<Option<i64>>,
}

impl DownloadQueueState {
    /// Keep displayed rows during reconnect, but require a new initial snapshot.
    pub fn disconnected(&self) {
        self.version.set(None);
    }

    /// False means the stream must restart with a fresh snapshot. Batches use
    /// absolute states, so overlapping and already-applied batches are safe.
    pub fn apply(&self, update: DownloadQueueUpdate) -> bool {
        if update.resync_required || update.from_version < 0 || update.version < update.from_version
        {
            return false;
        }
        if !update.snapshot {
            let Some(version) = self.version.get() else {
                return false;
            };
            if update.version <= version {
                return true;
            }
            if update.from_version > version {
                return false;
            }
        }

        let mut by_id = self.by_id.borrow_mut();
        let mut rows = self.rows.lock_mut();
        let mut ordering_changed = false;
        let removed: HashSet<_> = update.removed_ids.into_iter().collect();
        let retained: HashSet<_> = if update.snapshot {
            update
                .updates
                .iter()
                .map(|entry| entry.chapter_id)
                .collect()
        } else {
            HashSet::new()
        };
        if update.snapshot || !removed.is_empty() {
            rows.retain(|row| {
                let keep = !removed.contains(&row.chapter_id)
                    && (!update.snapshot || retained.contains(&row.chapter_id));
                if !keep {
                    by_id.remove(&row.chapter_id);
                }
                keep
            });
        }
        for entry in update.updates {
            if let Some(row) = by_id.get(&entry.chapter_id) {
                ordering_changed |=
                    row.order() != (entry.priority, entry.date_added, entry.chapter_id);
                row.data.set_neq(entry);
            } else {
                let row = Rc::new(DownloadQueueRow {
                    chapter_id: entry.chapter_id,
                    data: Mutable::new(entry),
                });
                by_id.insert(row.chapter_id, row.clone());
                rows.push_cloned(row);
                ordering_changed = true;
            }
        }
        if ordering_changed {
            let mut ordered: Vec<_> = rows.iter().cloned().collect();
            ordered.sort_by_key(|row| row.order());
            let mut positions: HashMap<_, _> = rows
                .iter()
                .enumerate()
                .map(|(index, row)| (row.chapter_id, index))
                .collect();
            for (index, row) in ordered.iter().enumerate() {
                let previous = positions[&row.chapter_id];
                if previous != index {
                    rows.move_from_to(previous, index);
                    for position in index.min(previous)..=index.max(previous) {
                        positions.insert(rows[position].chapter_id, position);
                    }
                }
            }
        }
        self.version.set(Some(update.version));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_signals::signal_vec::{SignalVec, VecDiff};
    use std::{
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    fn entry(id: i64, priority: i64) -> DownloadQueue {
        DownloadQueue {
            chapter_id: id,
            priority,
            total: 30,
            date_added: 100 - id,
            ..Default::default()
        }
    }

    fn update(
        snapshot: bool,
        from: i64,
        version: i64,
        entries: Vec<DownloadQueue>,
        removed: Vec<i64>,
    ) -> DownloadQueueUpdate {
        DownloadQueueUpdate {
            snapshot,
            from_version: from,
            version,
            updates: entries,
            removed_ids: removed,
            resync_required: false,
            download_status: true,
        }
    }

    #[test]
    fn progress_preserves_rows_and_emits_no_list_replacement() {
        let state = DownloadQueueState::default();
        assert!(state.apply(update(true, 0, 0, vec![entry(1, 0), entry(2, 1)], vec![])));
        let first = state.rows.lock_ref()[0].clone();
        let mut changes = state.rows.signal_vec_cloned();
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(
            Pin::new(&mut changes).poll_vec_change(&mut context),
            Poll::Ready(Some(VecDiff::Replace { .. }))
        ));
        let mut progress = entry(1, 0);
        progress.downloaded = 12;
        assert!(state.apply(update(false, 0, 12, vec![progress], vec![])));
        assert!(Rc::ptr_eq(&first, &state.rows.lock_ref()[0]));
        assert_eq!(first.data.lock_ref().downloaded, 12);
        assert!(
            Pin::new(&mut changes)
                .poll_vec_change(&mut context)
                .is_pending()
        );
    }

    #[test]
    fn overlapping_batches_and_reordering_preserve_row_identity() {
        let state = DownloadQueueState::default();
        assert!(state.apply(update(
            true,
            3,
            3,
            vec![entry(1, 0), entry(2, 1), entry(3, 2)],
            vec![]
        )));
        let first = state.rows.lock_ref()[0].clone();
        assert!(state.apply(update(false, 0, 5, vec![entry(1, 1), entry(2, 0)], vec![3])));
        assert_eq!(
            state
                .rows
                .lock_ref()
                .iter()
                .map(|row| row.chapter_id)
                .collect::<Vec<_>>(),
            [2, 1]
        );
        assert!(Rc::ptr_eq(&first, &state.rows.lock_ref()[1]));
        assert!(state.apply(update(false, 0, 5, vec![entry(3, -1)], vec![])));
        assert_eq!(state.rows.lock_ref().len(), 2);
    }

    #[test]
    fn gaps_require_resync_and_a_fresh_snapshot_accepts_version_zero() {
        let state = DownloadQueueState::default();
        assert!(state.apply(update(true, 10, 10, vec![entry(1, 0)], vec![])));
        assert!(!state.apply(update(false, 12, 15, vec![entry(2, 1)], vec![])));
        assert_eq!(state.rows.lock_ref().len(), 1);
        state.disconnected();
        assert!(!state.apply(update(false, 0, 1, vec![entry(2, 1)], vec![])));
        assert!(state.apply(update(true, 0, 0, vec![entry(2, 0)], vec![])));
        assert_eq!(state.rows.lock_ref()[0].chapter_id, 2);
        assert_eq!(state.version.get(), Some(0));
    }

    #[test]
    fn ordering_uses_priority_then_date_and_id() {
        let state = DownloadQueueState::default();
        assert!(state.apply(update(
            true,
            0,
            0,
            vec![entry(1, 10), entry(2, 0), entry(3, 0)],
            vec![]
        )));
        assert_eq!(
            state
                .rows
                .lock_ref()
                .iter()
                .map(|row| row.chapter_id)
                .collect::<Vec<_>>(),
            [3, 2, 1]
        );
    }
}
