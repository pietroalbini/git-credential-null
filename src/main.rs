use std::env;

fn main() {
    if let Some("get") = env::args().nth(1).as_ref().map(|s| s.as_str()) {
        println!("quit=1");
    }
}
