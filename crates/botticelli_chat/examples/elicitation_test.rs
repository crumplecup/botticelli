// Test example to explore elicitation crate API

use elicitation::{Elicit, Prompt, Select, Survey};

// Test Select paradigm
#[derive(Debug, Clone, Elicit)]
enum Priority {
    Low,
    Medium,
    High,
}

// Test Survey paradigm
#[derive(Debug, Clone, Elicit)]
struct Config {
    name: String,
    enabled: bool,
}

fn main() {
    println!("Elicitation API test compiled successfully!");
}
