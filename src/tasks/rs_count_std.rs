use std::hint::black_box;
use std::time::Instant;

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>>  {
    let start = Instant::now();
    let mut final_i = 0_i32;

    // `black_box` blocks LLVM from deleting the empty loop during optimization
    for i in 1..=1_000_000 {
        final_i = black_box(i);
    }

    let elapsed = start.elapsed();
    println!("STD Counter: {} | Tempo: {:.9}s", final_i, elapsed.as_secs_f64());
    Ok(())
}