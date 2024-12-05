use  clap::Parser;

#[derive(Parser)]
struct Args{
    files: Vec<String>,
    #[arg(short = 'n', long="number", help = "Show line number")]
    show_line_number: bool,
}
fn main() {
    println!("Hello, world!");
    if let Err(e) = run(Args::parse()){
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
   
}

fn run (args: Args) -> Result<(), Box<dyn std::error::Error>>{
    args.files.iter().try_for_each(|file| {
        let content = std::fs::read_to_string(file)?;
        content.lines().enumerate().for_each(|(index, line)| {
            if args.show_line_number {
                println!("{:4}: {}", index + 1, line);
            } else {
                println!("{}", line);
            }
        });
        Ok(())
    })
}
// test
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_run() {
        let args = Args {
            files: vec!["./src/main.rs".to_string()],
            show_line_number: true,
        };
        
        assert!(run(args).is_ok());
    }
}