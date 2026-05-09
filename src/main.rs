mod tasks;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let task_name = match args.get(1) {
        Some(name) => name.as_str(),
        None => {
            eprintln!("📦 Rust Task Hub");
            eprintln!("Uso: {} <task>", args[0]);

            // Stampa zero-alloc dal registro compile-time
            eprint!("Task disponibili: ");
            for (i, task) in tasks::TaskId::ALL.iter().enumerate() {
                if i > 0 { eprint!(", "); }
                eprint!("{}", task.name());
            }
            eprintln!();
            process::exit(1);
        }
    };

    match task_name.parse::<tasks::TaskId>() {
        Ok(id) => {
            if let Err(e) = tasks::execute(id) {
                eprintln!("❌ Errore esecuzione: {}", e);
                process::exit(1);
            }
        }
        Err(msg) => {
            eprintln!("❌ {}", msg);
            eprintln!("Task disponibili: ");
            for (i, task) in tasks::TaskId::ALL.iter().enumerate() {
                if i > 0 { eprint!(", "); }
                eprint!("{}", task.name());
            }
            eprintln!();
            process::exit(1);
        }
    }
}