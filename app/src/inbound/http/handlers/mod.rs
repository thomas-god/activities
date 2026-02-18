#![allow(unused_imports)]

mod activities;
mod auth;
mod preferences;
mod training;

pub(super) use activities::{
    delete_activity, get_activity, get_all_activities, list_activities, patch_activity,
    upload_activities,
};
pub(super) use auth::{login_user, register_user, validate_login};
pub(super) use preferences::{
    delete_preference, get_all_preferences, get_preference, set_preference,
};
pub(super) use training::{
    compute_training_metric_values, create_training_metric, create_training_note,
    create_training_period, delete_training_metric, delete_training_note, delete_training_period,
    get_active_training_periods, get_training_metric_values, get_training_metrics,
    get_training_metrics_ordering, get_training_note, get_training_notes, get_training_period,
    get_training_periods, set_training_metrics_ordering, update_training_metric,
    update_training_note, update_training_period,
};
