use crate::domain::models::Activity;

pub trait ActivityRepository: Clone + Send + Sync + 'static {
    fn save_activity(&self, activity: &Activity);
}
