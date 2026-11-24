#[cfg(any(test, feature = "test-utils"))]
use derive_more::Constructor;

#[cfg(any(test, feature = "test-utils"))]
use crate::ports::IClock;

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone, Constructor)]
pub struct FakeClock {
    time: chrono::DateTime<chrono::Utc>,
}

#[cfg(any(test, feature = "test-utils"))]
impl IClock for FakeClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.time
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeClock {
    fn default() -> Self {
        Self {
            time: chrono::Utc::now(),
        }
    }
}
