use fit_parser::parse_fit_messages;

fn main() {
    let messages = parse_fit_messages("fit-parser/examples/example.fit").unwrap();

    for message in messages {
        println!("{message:?}");
    }
}
