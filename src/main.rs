fn main() {
    // config
    let args = std::env::args().collect::<Vec<String>>();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        std::process::exit(1);
    });

    run(config).unwrap_or_else(|err| {
        eprintln!("Application error: {}", err);
        std::process::exit(1);
    });
}

fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // read file
    let contents = std::fs::read_to_string(config.path)?;
    println!("With text:\n{}", contents);

    // search
    // print
    Ok(())
}

struct Config {
    target: String,
    path: String,
}

// imple config new method
impl Config {
    fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }

        let target = args[1].clone();
        let path = args[2].clone();
        
        Ok(Config { target, path })
    }
}