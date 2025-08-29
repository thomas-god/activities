use fit_rs::parse_fit_messages;

fn main() {
    let messages = parse_fit_messages("fit-rs/examples/example.fit").unwrap();

    for message in messages {
        println!("{message:?}");
    }
}
