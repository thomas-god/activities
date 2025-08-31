use std::path::Path;

use codegen::generate_code;

fn main() {
    let profile = Path::new("Profile.xlsx");

    if !profile.exists() {
        println!("No Profile.xlsx found");
        return;
    }

    let code = generate_code(profile);

    std::fs::write("fit-rs/src/parser/types/generated.rs", code)
        .expect("Could not write to ouptut file at src/parser/types/generated.rs");

    println!("Code successfully generated at fit-rs/src/parser/types/generated.rs")
}
