use std::env;
use std::fs;
use executor::handler::execute_sql_and_get_output;
use storage::storage::SimpleStorageEngine;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <sql_input_file>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let input_sql = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", input_path, e);
            std::process::exit(1);
        }
    };

    // 使用当前目录的 data.json 作为持久化路径
    let mut engine = SimpleStorageEngine::new("data.json");

    match execute_sql_and_get_output(&input_sql, &mut engine) {
        Ok(output) => {
            if output.trim().is_empty() {
                println!("There are no results to be displayed.");
            } else {
                println!("{}", output);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
