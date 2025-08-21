use activities::parse_records;

fn main() {
    let records = parse_records("dev_data.fit").unwrap();
    for record in records.iter() {
        println!("{record:?}");
    }
}
