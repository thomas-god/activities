use std::path::Path;

use codegen::generate_code;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let profile = Path::new("../Profile.xlsx");
    let code = generate_code(profile);

    std::fs::write("src/parser/types/generated.rs", code).expect("Could not wirte to ouptut file");
}
