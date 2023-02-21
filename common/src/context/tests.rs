use std::time::Duration;

use super::*;

#[tokio::test]
async fn test_context_cancel() {
    let (ctx, handler) = Context::new();

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Cancel);
    });

    tokio::time::timeout(Duration::from_millis(300), handler.cancel())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handle)
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

    tokio::time::timeout(Duration::from_millis(300), handle)
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    tokio::time::timeout(Duration::from_millis(300), handler.done())
        .await
        .expect("task should be cancelled");
}

#[tokio::test]
async fn test_context_timeout() {
    let (ctx, mut handler) = Context::with_timeout(Duration::from_millis(100));

    let handle = tokio::spawn(async move {
        let reason = ctx.done().await;
        assert_eq!(reason, CancelReason::Deadline);
    });

    tokio::time::timeout(Duration::from_millis(300), handle)
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    tokio::time::timeout(Duration::from_millis(300), handler.done())
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

    tokio::time::timeout(Duration::from_millis(300), parent_handler.cancel())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handle)
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    tokio::time::timeout(Duration::from_millis(300), handler.done())
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

    tokio::time::timeout(Duration::from_millis(300), parent_handler.done())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handler.done())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handle)
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

    tokio::time::timeout(Duration::from_millis(300), handler.cancel())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), parent_handler.done())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handle)
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

    tokio::time::timeout(Duration::from_millis(300), parent_handler.cancel())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handler.done())
        .await
        .expect("task should be cancelled");
    tokio::time::timeout(Duration::from_millis(300), handle)
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

    tokio::time::timeout(Duration::from_millis(300), handler.cancel())
        .await
        .expect_err("task should block because a clone exists");
    tokio::time::timeout(Duration::from_millis(300), handle)
        .await
        .expect("task should be cancelled")
        .expect("panic in task");
    tokio::time::timeout(Duration::from_millis(300), ctx2.done())
        .await
        .expect("task should be cancelled");
}

#[test]
fn test_fmt_reason() {
    assert_eq!(format!("{}", CancelReason::Cancel), "Cancel");
    assert_eq!(format!("{}", CancelReason::Deadline), "Deadline");
    assert_eq!(format!("{}", CancelReason::Parent), "Parent");
}
