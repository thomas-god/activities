use derive_more::Constructor;

use crate::domain::shared::IClock;

#[derive(Debug, Clone, Constructor)]
pub struct Clock;

impl IClock for Clock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}
