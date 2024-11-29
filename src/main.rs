

fn main() {
    // config
    let args = std::env::args().collect::<Vec<String>>();

    let config = rust_demo::Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        std::process::exit(1);
    });

    rust_demo::run(config).unwrap_or_else(|err| {
        eprintln!("Application error: {}", err);
        std::process::exit(1);
    });
}

