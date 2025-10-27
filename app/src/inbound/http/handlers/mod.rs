#![allow(unused_imports)]

mod create_training_metric;
mod create_training_note;
mod create_training_period;
mod delete_activity;
mod delete_training_metric;
mod delete_training_period;
mod get_activity;
mod get_training_metrics;
mod get_training_period;
mod get_training_periods;
mod list_activities;
mod login_user;
mod patch_activity;
mod register_user;
mod types;
mod update_training_period;
mod upload_activity;
mod validate_login;

pub(super) use create_training_metric::create_training_metric;
pub(super) use create_training_note::create_training_note;
pub(super) use create_training_period::create_training_period;
pub(super) use delete_activity::delete_activity;
pub(super) use delete_training_metric::delete_training_metric;
pub(super) use delete_training_period::delete_training_period;
pub(super) use get_activity::get_activity;
pub(super) use get_training_metrics::get_training_metrics;
pub(super) use get_training_period::get_training_period;
pub(super) use get_training_periods::get_training_periods;
pub(super) use list_activities::list_activities;
pub(super) use login_user::login_user;
pub(super) use patch_activity::patch_activity;
pub(super) use register_user::register_user;
pub(super) use update_training_period::update_training_period_name;
pub(super) use upload_activity::upload_activities;
pub(super) use validate_login::validate_login;
