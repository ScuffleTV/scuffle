use std::time::Duration;

use tokio::time::Instant;

use crate::{
    context::{CancelReason, Context},
    prelude::FutureTimeout,
};

#[tokio::test]
async fn test_context_cancel() {
    let (ctx, handler) = Context::new();

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Cancel);
    });

    handler
        .cancel()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
}

#[tokio::test]
async fn test_context_deadline() {
    let (ctx, mut handler) = Context::with_deadline(Instant::now() + Duration::from_millis(100));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Deadline);
    });

    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
}

#[tokio::test]
async fn test_context_is_done() {
    let (ctx, handler) = Context::new();

    let handle = tokio::spawn(async move {
        assert!(!ctx.is_done());
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Cancel);
        assert!(ctx.is_done());
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    drop(handler);
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
}

#[tokio::test]
async fn test_context_timeout() {
    let (ctx, mut handler) = Context::with_timeout(Duration::from_millis(100));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Deadline);
    });

    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
}

#[tokio::test]
async fn test_context_parent() {
    let (parent, parent_handler) = Context::new();
    let (ctx, mut handler) = Context::with_parent(parent, None);

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Parent);
    });

    parent_handler
        .cancel()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
}

#[tokio::test]
async fn test_context_parent_deadline() {
    let (parent, mut parent_handler) = Context::new();
    let (ctx, mut handler) =
        Context::with_parent(parent, Some(Instant::now() + Duration::from_millis(100)));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Deadline);
    });

    parent_handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
}

#[tokio::test]
async fn test_context_parent_deadline_cancel() {
    let (parent, mut parent_handler) = Context::new();
    let (ctx, handler) =
        Context::with_parent(parent, Some(Instant::now() + Duration::from_millis(100)));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Cancel);
    });

    handler
        .cancel()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    parent_handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
}

#[tokio::test]
async fn test_context_parent_deadline_parent_cancel() {
    let (parent, parent_handler) = Context::new();
    let (ctx, mut handler) =
        Context::with_parent(parent, Some(Instant::now() + Duration::from_millis(100)));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Parent);
    });

    parent_handler
        .cancel()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handler
        .done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
}

#[tokio::test]
async fn test_context_cancel_cloned() {
    let (ctx, handler) = Context::new();
    let ctx2 = ctx.clone();

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Cancel);
    });

    handler
        .cancel()
        .timeout(Duration::from_millis(300))
        .await
        .expect_err("task should block because a clone exists");
    handle
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    ctx2.done()
        .timeout(Duration::from_millis(300))
        .await
        .expect("task should be cancelled");
}

#[test]
fn test_fmt_reason() {
    assert_eq!(format!("{}", CancelReason::Cancel), "Cancel");
    assert_eq!(format!("{}", CancelReason::Deadline), "Deadline");
    assert_eq!(format!("{}", CancelReason::Parent), "Parent");
}
