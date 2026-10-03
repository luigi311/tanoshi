use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use super::{
    RequestPriority::{High, Low},
    RequestQueue,
};

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn high_priority_runs_first_with_fifo_within_each_priority() {
    let queue = RequestQueue::new(2);
    let active_one = queue.acquire(Low).await.unwrap();
    let active_two = queue.acquire(Low).await.unwrap();
    let mut low_one = Box::pin(queue.acquire(Low));
    let mut low_two = Box::pin(queue.acquire(Low));
    let mut high_one = Box::pin(queue.acquire(High));
    let mut high_two = Box::pin(queue.acquire(High));
    assert!(poll_once(low_one.as_mut()).is_pending());
    assert!(poll_once(low_two.as_mut()).is_pending());
    assert!(poll_once(high_one.as_mut()).is_pending());
    assert!(poll_once(high_two.as_mut()).is_pending());

    drop(active_one);
    assert!(poll_once(low_one.as_mut()).is_pending());
    assert!(poll_once(high_two.as_mut()).is_pending());
    let high_one = high_one.await.unwrap();
    drop(active_two);
    let high_two = high_two.await.unwrap();
    assert!(poll_once(low_one.as_mut()).is_pending());

    drop(high_one);
    assert!(poll_once(low_two.as_mut()).is_pending());
    let low_one = low_one.await.unwrap();
    drop(high_two);
    let low_two = low_two.await.unwrap();
    drop((low_one, low_two));
    // All slots are recovered after the full queue drains.
    let one = queue.acquire(High).await.unwrap();
    let two = queue.acquire(High).await.unwrap();
    drop((one, two));
}

#[tokio::test]
async fn cancellation_before_and_after_grant_returns_capacity() {
    let queue = RequestQueue::new(1);
    let active = queue.acquire(Low).await.unwrap();
    let mut cancelled_waiter = Box::pin(queue.acquire(High));
    let mut cancelled_grant = Box::pin(queue.acquire(High));
    let mut next = Box::pin(queue.acquire(Low));
    assert!(poll_once(cancelled_waiter.as_mut()).is_pending());
    assert!(poll_once(cancelled_grant.as_mut()).is_pending());
    assert!(poll_once(next.as_mut()).is_pending());
    drop(cancelled_waiter);
    drop(active);
    // cancelled_grant owns the slot but has not resumed since being woken.
    assert!(poll_once(next.as_mut()).is_pending());
    drop(cancelled_grant);
    drop(next.await.unwrap());
    drop(queue.acquire(High).await.unwrap());
}

#[tokio::test]
async fn closing_wakes_both_priorities_and_rejects_new_requests() {
    let queue = RequestQueue::new(1);
    let active = queue.acquire(High).await.unwrap();
    let mut high = Box::pin(queue.acquire(High));
    let mut low = Box::pin(queue.acquire(Low));
    assert!(poll_once(high.as_mut()).is_pending());
    assert!(poll_once(low.as_mut()).is_pending());
    queue.close();
    assert!(high.await.is_err());
    assert!(low.await.is_err());
    drop(active);
    assert!(queue.acquire(High).await.is_err());
    assert!(queue.acquire(Low).await.is_err());
}
