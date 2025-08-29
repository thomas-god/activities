use fit_rs::{
    DeviceInfoField, FitMessage, parse_fit_messages,
    utils::{find_fied_value_as_string, find_field_value_by_kind},
};

fn main() {
    let messages = parse_fit_messages("fit-rs/examples/example.fit").unwrap();

    let device = find_fied_value_as_string(
        &messages,
        &FitMessage::DeviceInfo(DeviceInfoField::ProductName),
    );
    println!("Device name: {device:?}");

    let sport =
        find_field_value_by_kind(&messages, &FitMessage::Session(fit_rs::SessionField::Sport));
    println!("Sport: {sport:?}");

    let calories = find_field_value_by_kind(
        &messages,
        &FitMessage::Session(fit_rs::SessionField::TotalCalories),
    );
    println!("Calories: {calories:?} kcal");
}
