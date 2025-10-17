#![allow(unused_imports)]

mod create_training_metric;
mod create_training_period;
mod delete_activity;
mod delete_training_metric;
mod get_activity;
mod get_training_metrics;
mod get_training_periods;
mod list_activities;
mod login_user;
mod patch_activity;
mod register_user;
mod types;
mod upload_activity;
mod validate_login;

pub(super) use create_training_metric::create_training_metric;
pub(super) use create_training_period::create_training_period;
pub(super) use delete_activity::delete_activity;
pub(super) use delete_training_metric::delete_training_metric;
pub(super) use get_activity::get_activity;
pub(super) use get_training_metrics::get_training_metrics;
pub(super) use get_training_periods::get_training_periods;
pub(super) use list_activities::list_activities;
pub(super) use login_user::login_user;
pub(super) use patch_activity::patch_activity;
pub(super) use register_user::register_user;
pub(super) use upload_activity::upload_activities;
pub(super) use validate_login::validate_login;
