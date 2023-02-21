use std::time::Duration;

use tokio::process::Command;

use super::*;

#[tokio::test]
async fn test_signal() {
    let mut handler = SignalHandler::new()
        .with_signal(SignalKind::interrupt())
        .with_signal(SignalKind::terminate());

    // Send a SIGINT to the process
    // We need to get the current pid and send the signal to it
    let pid = std::process::id();

    Command::new("kill")
        .arg("-s")
        .arg("SIGINT")
        .arg(pid.to_string())
        .status()
        .await
        .expect("failed to send SIGINT");

    tokio::time::timeout(Duration::from_secs(1), handler.recv())
        .await
        .expect("failed to receive signal");

    // Send a SIGTERM to the process
    Command::new("kill")
        .arg("-s")
        .arg("SIGTERM")
        .arg(pid.to_string())
        .status()
        .await
        .expect("failed to send SIGINT");

    tokio::time::timeout(Duration::from_secs(1), handler.recv())
        .await
        .expect("failed to receive signal");
}
