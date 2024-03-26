use std::ops::ControlFlow;
use std::time::Duration;

/// a simple **Reconnection** Handler.
///
/// with implementing this trait and passing it to the `InitParams` inside the `Client` you can have your own
/// custom implementations for handling connection failures.
///
/// the default implementation is **NoReconnect** which does not handle anything! there is also a `FixedReconnect`
/// which sets a fixed attempt count and a duration
///
/// note that this will return a `ControlFlow<(), Duration>` which tells the handler either `Break` the Connection Attempt *or*
/// `Continue` After the Given `Duration`
pub trait RetryPolicy: Send + Sync {
    ///this function will indicate that the handler should attempt for a new *reconnection* or not.
    ///
    /// it accepts a `attempts` which is the amount of reconnection tries that has been made already
    fn should_retry(&self, attempts: usize) -> ControlFlow<(), Duration>;
}

/// the default implementation of the **ReconnectionPolicy**.
pub struct NoRetry;

/// simple *Fixed* sized implementation for the **ReconnectionPolicy** trait.
pub struct Fixed {
    pub attempts: usize,
    pub delay: Duration,
}

impl Fixed {
    pub const fn new(attempts: usize, delay: Duration) -> Self {
        Self { attempts, delay }
    }
}

impl RetryPolicy for Fixed {
    fn should_retry(&self, attempts: usize) -> ControlFlow<(), Duration> {
        if attempts <= self.attempts {
            ControlFlow::Continue(self.delay)
        } else {
            ControlFlow::Break(())
        }
    }
}

impl RetryPolicy for NoRetry {
    fn should_retry(&self, _: usize) -> ControlFlow<(), Duration> {
        ControlFlow::Break(())
    }
}

#[macro_export]
macro_rules! retrying {
    ($policy:expr, $body:expr) => {{
        let mut attempts = 0;
        loop {
            let res = $body;
            attempts += 1;
            match res {
                Ok(value) => {
                    break Ok(value);
                }
                Err(_) => match $policy.should_retry(attempts) {
                    std::ops::ControlFlow::Continue(timeout) => {
                        tokio::time::sleep(timeout).await;
                        continue;
                    }
                    std::ops::ControlFlow::Break(_) => break res,
                },
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Erroring {
        cnt: usize,
    }

    impl Erroring {
        fn new(cnt: usize) -> Self {
            Self { cnt }
        }

        async fn run(&mut self) -> Result<usize, usize> {
            if self.cnt > 0 {
                self.cnt -= 1;
                return Err(self.cnt);
            }
            Ok(self.cnt)
        }
    }

    #[tokio::test]
    async fn test_retrying_macro_ok() {
        let policy = Fixed::new(10, Duration::new(0, 0));
        let mut err = Erroring::new(5);

        let r = retrying!(policy, err.run().await);
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_retrying_macro_err() {
        let policy = Fixed::new(3, Duration::new(0, 0));
        let mut err = Erroring::new(5);

        let r = retrying!(policy, err.run().await);
        assert!(r.is_err());
    }
}
