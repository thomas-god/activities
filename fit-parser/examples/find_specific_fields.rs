use fit_parser::{
    DeviceInfoField, FitField, SessionField, parse_fit_file,
    utils::{find_fied_value_as_string, find_field_value_by_kind},
};

fn main() {
    let messages = parse_fit_file("fit-parser/examples/example.fit", false).unwrap();

    let device = find_fied_value_as_string(
        &messages,
        &FitField::DeviceInfo(DeviceInfoField::ProductName),
    );
    println!("Device name: {device:?}");

    let sport = find_field_value_by_kind(&messages, &FitField::Session(SessionField::Sport));
    println!("Sport: {sport:?}");

    let calories =
        find_field_value_by_kind(&messages, &FitField::Session(SessionField::TotalCalories));
    println!("Calories: {calories:?} kcal");
}
