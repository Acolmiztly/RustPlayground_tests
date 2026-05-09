#[inline(always)]
pub fn count_parallel_asm(limit: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let mut a: u32; let mut b: u32;
        let mut c: u32; let mut d: u32;
        let mut iter: u32; let mut tail: u32;

        unsafe {
            core::arch::asm!(
            "xor {a}, {a}", "xor {b}, {b}",
            "xor {c}, {c}", "xor {d}, {d}",
            "mov {iter}, {lim}",
            "mov {tail}, {lim}",
            "and {tail}, 3",
            "2:",
            "  test {tail}, {tail}",
            "  jz 3f",
            "  add {a}, 1",
            "  sub {tail}, 1",
            "  jmp 2b",
            "3:",
            "  shr {iter}, 2",
            "  test {iter}, {iter}",
            "  jz 4f",
            "5:",
            "  add {a}, 1", "add {b}, 1",
            "  add {c}, 1", "add {d}, 1",
            "  sub {iter}, 1",
            "  jnz 5b",
            "4:",
            "  add {a}, {b}",
            "  add {c}, {d}",
            "  add {a}, {c}",
            a = out(reg) a, b = out(reg) b,
            c = out(reg) c, d = out(reg) d,
            iter = out(reg) iter, tail = out(reg) tail,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        a
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mut a: u32; let mut b: u32;
        let mut c: u32; let mut d: u32;
        let mut iter: u32; let mut tail: u32;

        // SAFETY: Idem. Only register-to-register ops.
        unsafe {
            core::arch::asm!(
            "mov {a:w}, #0", "mov {b:w}, #0",
            "mov {c:w}, #0", "mov {d:w}, #0",
            "mov {tail:w}, {lim:w}",
            "and {tail:w}, {tail:w}, #3",
            "2:",
            "  cbz {tail:w}, 3f",
            "  add {a:w}, {a:w}, 1",
            "  sub {tail:w}, {tail:w}, 1",
            "  b 2b",
            "3:",
            "  mov {iter:w}, {lim:w}",
            "  lsr {iter:w}, {iter:w}, 2",
            "  cbz {iter:w}, 5f",
            "4:",
            "  add {a:w}, {a:w}, 1",
            "  add {b:w}, {b:w}, 1",
            "  add {c:w}, {c:w}, 1",
            "  add {d:w}, {d:w}, 1",
            "  subs {iter:w}, {iter:w}, 1",
            "  b.ne 4b",
            "5:",
            "  add {a:w}, {a:w}, {b:w}",
            "  add {c:w}, {c:w}, {d:w}",
            "  add {a:w}, {a:w}, {c:w}",
            a = out(reg) a, b = out(reg) b,
            c = out(reg) c, d = out(reg) d,
            iter = out(reg) iter, tail = out(reg) tail,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        a
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let mut a = 0; let mut b = 0;
        let mut c = 0; let mut d = 0;
        let mut i = 0;
        while i + 4 <= limit {
            a += 1; b += 1; c += 1; d += 1;
            i += 4;
        }
        while i < limit { a += 1; i += 1; }
        a + b + c + d
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limit = 1_000_000_u32;
    let start = std::time::Instant::now();
    let res = count_parallel_asm(limit);
    let elapsed = start.elapsed();

    println!("Parallel ASM: {} | {:.9}s | {:.6}M iter/s",
             res, elapsed.as_secs_f64(),
             (limit as f64) / elapsed.as_secs_f64() / 1_000_000.0);
    Ok(())
}