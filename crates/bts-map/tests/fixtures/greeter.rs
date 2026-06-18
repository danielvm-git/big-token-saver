// Fixture: Rust greeter (bts-map snapshot test)
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

struct Greeter {
    prefix: String,
}

impl Greeter {
    fn new(prefix: &str) -> Self {
        Greeter { prefix: prefix.to_string() }
    }

    fn greet(&self, name: &str) -> String {
        format!("{} {}!", self.prefix, name)
    }
}

fn main() {
    let g = Greeter::new("Hello");
    println!("{}", g.greet("world"));
    greet("raw");
}
