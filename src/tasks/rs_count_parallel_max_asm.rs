#[inline(always)]
pub fn count_ultra_fast_asm(limit: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]                                                                  //Conditional block based on sys architecture
    {
        let mut a: u32; let mut b: u32; let mut c: u32; let mut d: u32;
        let mut e: u32; let mut f: u32; let mut g: u32; let mut h: u32;
        let mut cnt: u32; let mut rem: u32;

        unsafe {
            core::arch::asm!(
            "xor {a}, {a}", "xor {b}, {b}", "xor {c}, {c}", "xor {d}, {d}",
            "xor {e}, {e}", "xor {f}, {f}", "xor {g}, {g}", "xor {h}, {h}",
            "mov {cnt}, {lim}",
            "mov {rem}, {lim}",
            "and {rem}, 7",          // rem = limit % 8
            "2:",                    // Remainder loop (max 7 iter)
            "  test {rem}, {rem}",
            "  jz 3f",
            "  add {a}, 1",
            "  dec {rem}",
            "  jmp 2b",
            "3:",                    // Main loop setup
            "  shr {cnt}, 3",        // cnt = limit / 8
            "  test {cnt}, {cnt}",
            "  jz 4f",
            "5:",                    // Unrolled 8x loop
            "  add {a}, 1", "add {b}, 1", "add {c}, 1", "add {d}, 1",
            "  add {e}, 1", "add {f}, 1", "add {g}, 1", "add {h}, 1",
            "  dec {cnt}",
            "  jnz 5b",
            "4:",                    // Horizontal reduction (3 livelli)
            "  add {a}, {b}", "add {c}, {d}",
            "  add {e}, {f}", "add {g}, {h}",
            "  add {a}, {c}", "add {e}, {g}",
            "  add {a}, {e}",
            a = out(reg) a, b = out(reg) b, c = out(reg) c, d = out(reg) d,
            e = out(reg) e, f = out(reg) f, g = out(reg) g, h = out(reg) h,
            cnt = out(reg) cnt, rem = out(reg) rem,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        a
    }

    #[cfg(target_arch = "aarch64")]                                                                 //Conditional block based on sys architecture
    {
        let mut a: u32; let mut b: u32; let mut c: u32; let mut d: u32;
        let mut e: u32; let mut f: u32; let mut g: u32; let mut h: u32;
        let mut cnt: u32; let mut rem: u32;

        // SAFETY: Idem. Only register-to-register ops.
        unsafe {
            core::arch::asm!(
            "mov {a:w}, #0", "mov {b:w}, #0", "mov {c:w}, #0", "mov {d:w}, #0",
            "mov {e:w}, #0", "mov {f:w}, #0", "mov {g:w}, #0", "mov {h:w}, #0",
            "mov {rem:w}, {lim:w}",
            "and {rem:w}, {rem:w}, #7",
            "2:",
            "  cbz {rem:w}, 3f",
            "  add {a:w}, {a:w}, 1",
            "  sub {rem:w}, {rem:w}, 1",
            "  b 2b",
            "3:",
            "  mov {cnt:w}, {lim:w}",
            "  lsr {cnt:w}, {cnt:w}, 3",
            "  cbz {cnt:w}, 5f",
            "4:",
            "  add {a:w}, {a:w}, 1", "add {b:w}, {b:w}, 1",
            "  add {c:w}, {c:w}, 1", "add {d:w}, {d:w}, 1",
            "  add {e:w}, {e:w}, 1", "add {f:w}, {f:w}, 1",
            "  add {g:w}, {g:w}, 1", "add {h:w}, {h:w}, 1",
            "  subs {cnt:w}, {cnt:w}, 1",
            "  b.ne 4b",
            "5:",
            "  add {a:w}, {a:w}, {b:w}", "add {c:w}, {c:w}, {d:w}",
            "  add {e:w}, {e:w}, {f:w}", "add {g:w}, {g:w}, {h:w}",
            "  add {a:w}, {a:w}, {c:w}", "add {e:w}, {e:w}, {g:w}",
            "  add {a:w}, {a:w}, {e:w}",
            a = out(reg) a, b = out(reg) b, c = out(reg) c, d = out(reg) d,
            e = out(reg) e, f = out(reg) f, g = out(reg) g, h = out(reg) h,
            cnt = out(reg) cnt, rem = out(reg) rem,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        a
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]                               //Conditional block based on non-implemented sys architecture
    {
        let mut acc = [0u32; 8];
        let mut i = 0;
        while i + 8 <= limit {
            acc[0] += 1; acc[1] += 1; acc[2] += 1; acc[3] += 1;
            acc[4] += 1; acc[5] += 1; acc[6] += 1; acc[7] += 1;
            i += 8;
        }
        while i < limit { acc[0] += 1; i += 1; }
        acc.iter().sum()
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limit = 1_000_000_u32;
    let start = std::time::Instant::now();
    let res = count_ultra_fast_asm(limit);
    let elapsed = start.elapsed();

    println!("Parallel ASM (max): {} | {:.9}s | {:.6}M iter/s",
             res, elapsed.as_secs_f64(),
             (limit as f64) / elapsed.as_secs_f64() / 1_000_000.0);
    Ok(())
}