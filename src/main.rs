use std::env;

fn main() {
    if let Some("get") = env::args().skip(1).next().as_ref().map(|s| s.as_str()) {
        println!("quit=1");
    }
}
