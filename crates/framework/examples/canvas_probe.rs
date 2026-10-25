//! Optional manual integration check; token is read from stdin, never logged.
use planner_framework::external::{save_canvas_token, CanvasReader, ExternalReader};
use std::io::Read;
fn main() {
    let mut token = String::new();
    std::io::stdin().read_to_string(&mut token).expect("stdin");
    save_canvas_token(token.trim()).expect("keychain");
    drop(token);
    match CanvasReader.list("/api/v1/courses?per_page=100") {
        Ok(courses) => println!(
            "Authenticated; {} course records; token stored in Keychain",
            courses.len()
        ),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
