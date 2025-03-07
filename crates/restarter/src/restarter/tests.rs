use std::time::Duration;

use anyhow::Result;
use futures::FutureExt;

use crate::restarter::{Restart, Restarter};

mockall::mock! {
    #[derive(Debug)]
    Restart {}
    impl Clone for Restart {
        fn clone(&self) -> Self;
    }
    impl Restart for Restart {
        fn restart(&self) -> impl Future<Output = Result<()>> + Send;
    }
}

#[tokio::test(start_paused = true)]
async fn restart_as_scheduled() {
    let mut mock_restart = MockRestart::new();
    mock_restart
        .expect_clone()
        .once()
        .return_once(move || {
            let mut mock_restart = MockRestart::new();
            mock_restart
                .expect_restart()
                .once()
                .returning(|| async { Ok(()) }.boxed());

            mock_restart
        });

    let restarter = Restarter::new(Duration::from_secs(300), mock_restart);

    let wait = restarter.schedule_restart().await;

    restarter.set_connection_count(0).await;

    assert!(wait.await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn cancel_restart() {
    let mut mock_restart = MockRestart::new();
    mock_restart
        .expect_clone()
        .once()
        .return_once(move || {
            let mut mock_restart = MockRestart::new();
            mock_restart
                .expect_restart()
                .never();

            mock_restart
        });

    let restarter = Restarter::new(Duration::from_secs(300), mock_restart);

    let wait = restarter.schedule_restart().await;

    restarter.set_connection_count(1).await;

    assert!(wait.await.is_ok());
}
