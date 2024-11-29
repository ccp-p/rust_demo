pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // read file
    let contents = std::fs::read_to_string(config.path)?;
    println!("With text:\n{}", contents);

    // search
    // print
    Ok(())
}

// Config struct
#[derive(Debug,PartialEq)]
pub struct Config {
    target: String,
    path: String,
}

// imple config new method

impl Config {
   pub fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }

        let target = args[1].clone();
        let path = args[2].clone();
        
        Ok(Config { target, path })
    }
  pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {

    contents.lines()
        .filter(|line| line.to_lowercase().contains(&query.to_lowercase()))
        .collect()
  }    
  pub fn search_case_sensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents.lines()
        .filter(|line| line.contains(query))
        .collect()
}
}

// tdd
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let args = vec![
            String::from("program_name"), // 通常第一个参数是程序名称
            String::from("target"),
            String::from("path"),
        ];
        let config = Config::new(&args).unwrap();
        assert_eq!(config.target, "target");
        assert_eq!(config.path, "path");
    }

    #[test]
    fn test_new_config_not_enough_args() {
        let args = vec![
            String::from("target"),
        ];
        let config = Config::new(&args);
        assert_eq!(config, Err("not enough arguments"));
    }
    #[test] 
    // case_sensitive
    fn test_search_case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";
        assert_eq!(
            vec!["safe, fast, productive."],
            Config::search_case_insensitive(query, contents)
        );
        
        }
    #[test]
    // case_insensitive
    fn test_search_case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";
        assert_eq!(
            vec!["Rust:", "Trust me."],
            Config::search_case_insensitive(query, contents)
        );
    }

}