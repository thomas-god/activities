use fit_parser::parse_fit_file;

fn main() {
    let messages = parse_fit_file("fit-parser/examples/example.fit").unwrap();

    for message in messages {
        println!("{message:?}");
    }
}
