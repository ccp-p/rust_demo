use  clap::Parser;

#[derive(Parser)]
struct Args{
    files: Vec<String>
    
}
fn main() {
    println!("Hello, world!");
    if let Err(e) = run(Args::parse()){
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
   
}

fn run (args: Args) -> Result<(), Box<dyn std::error::Error>>{
    let mut line_num = 1;

    for file in args.files{
        let content = std::fs::read_to_string(&file)?;
        for line in content.lines(){
            println!("{:4}: {}", line_num, line);
            line_num += 1;
        }
    }
    Ok(())
}