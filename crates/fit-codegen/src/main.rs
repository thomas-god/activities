use std::path::Path;

use fit_codegen::generate_code;

fn main() {
    let profile = Path::new("Profile.xlsx");

    if !profile.exists() {
        println!("No Profile.xlsx found");
        return;
    }

    let code = generate_code(profile);

    std::fs::write("crates/fit-parser/src/parser/types/generated.rs", code).expect(
        "Could not write to ouptut file at crates/fit-parser/src/parser/types/generated.rs",
    );

    println!("Code successfully generated at crates/fit-parser/src/parser/types/generated.rs")
}
