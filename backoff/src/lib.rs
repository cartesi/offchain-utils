use std::time::Duration;
use tokio::time::sleep;

use rand::thread_rng;
use rand::Rng;

pub struct Backoff {
    max_delay: Duration,
    max_retry: usize,
    current_retry: usize,
}

impl Backoff {
    pub fn new(max_retry: usize, max_delay: Duration) -> Self {
        Backoff {
            max_retry,
            max_delay,
            current_retry: 0,
        }
    }

    pub async fn wait(&mut self) -> Result<(), ()> {
        if self.current_retry >= self.max_retry {
            return Err(());
        }

        let jitter = {
            let mut rng = thread_rng();
            let ms: u64 = rng.gen_range(0..1000);
            Duration::from_millis(ms)
        };

        let current_wait = Duration::from_secs(1 << self.current_retry);
        let total_wait = current_wait + jitter;
        let capped_time = std::cmp::min(total_wait, self.max_delay);

        sleep(capped_time).await;

        self.current_retry += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Backoff;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn wait_test() {
        let mut backoff = Backoff::new(5, Duration::from_secs(32));

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 1000);
        assert!(elapsed <= 2000);

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 2000);
        assert!(elapsed <= 3000);

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 4000);
        assert!(elapsed <= 5000);
    }

    #[tokio::test]
    async fn max_wait_test() {
        let mut backoff = Backoff::new(5, Duration::from_secs(3));

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 1000);
        assert!(elapsed <= 2000);

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 2000);
        assert!(elapsed <= 3000);

        let now = Instant::now();
        backoff.wait().await.unwrap();
        let elapsed = now.elapsed().as_millis();
        assert!(elapsed >= 3000);
        assert!(elapsed <= 3100);
    }

    #[tokio::test]
    async fn retry_test1() {
        let mut backoff = Backoff::new(0, Duration::from_secs(32));
        assert!(backoff.wait().await.is_err());
    }

    #[tokio::test]
    async fn retry_test2() {
        let mut backoff = Backoff::new(2, Duration::from_secs(32));
        assert!(backoff.wait().await.is_ok());
        assert!(backoff.wait().await.is_ok());
        assert!(backoff.wait().await.is_err());
    }
}
