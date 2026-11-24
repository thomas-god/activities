use derive_more::Constructor;

use crate::domain::ports::IClock;

#[derive(Debug, Clone, Constructor)]
pub struct Clock;

impl IClock for Clock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

#[cfg(test)]
pub mod clock_test_utils {
    use derive_more::Constructor;

    use crate::domain::ports::IClock;

    #[derive(Debug, Clone, Constructor)]
    pub struct FakeClock {
        time: chrono::DateTime<chrono::Utc>,
    }

    impl IClock for FakeClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.time
        }
    }

    impl Default for FakeClock {
        fn default() -> Self {
            Self {
                time: chrono::Utc::now(),
            }
        }
    }
}
