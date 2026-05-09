
/// Conta da 0 a `limit - 1` usando assembly inline ottimizzato per ISA.
/// Restituisce il valore finale del contatore.
#[inline(always)]
pub fn count_asm(limit: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let mut res: u32;
        // SAFETY: The asm access only declared registers.
        // It doesn't manipulate memory, stack pointer or Rust's critical flags
        
        unsafe {
            core::arch::asm!(
            "xor {res}, {res}",        // res = 0
            "mov {cnt}, {lim}",        // cnt = limit
            "2:",                      // numeric label start progression from "2" to avoid significant numbers ( 0 , 1 ) and non-allowed chars for in-line asm
            "  add {res}, 1",          // res++
            "  sub {cnt}, 1",          // cnt--
            "  jnz 2b",                // jump if not zero (ZF)
            res = out(reg) res,
            cnt = out(reg) _,
            lim = in(reg) limit,
            options(nostack)           // Tells the compiler that the stack frame is invariant
            );
        }
        res
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mut res: u32;
        // SAFETY: Idem. `cbnz` reads only the declared register, no memory.
        unsafe {
            core::arch::asm!(
            "mov {res}, #0",
            "mov {cnt}, {lim}",
            "1:",
            "  add {res:w}, {res:w}, 1",
            "  sub {cnt:w}, {cnt:w}, 1",
            "  cbnz {cnt:w}, 1b",           // compare & branch if non-zero
            res = out(reg) res,
            cnt = out(reg) _,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        res
    }

    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    {
        let mut res: u32;
        // SAFETY: Idem. `bnez` is native RISC-V, zero side effect on memory/stack.
        unsafe {
            core::arch::asm!(
            "li {res}, 0",
            "mv {cnt}, {lim}",
            "1:",
            "  addi {res}, {res}, 1",
            "  addi {cnt}, {cnt}, -1",
            "  bnez {cnt}, 1b",        // branch if not zero
            res = out(reg) res,
            cnt = out(reg) _,
            lim = in(reg) limit,
            options(nostack)
            );
        }
        res
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64", target_arch = "riscv32")))]
    {
        // Fallback: LLVM optimized for non-implemented architectures
        let mut res = 0_u32;
        for _ in 0..limit {
            res = std::hint::black_box(res + 1);
        }
        res
    }
}
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let limit = 1_000_000_u32;
    let start = std::time::Instant::now();
    let res = count_asm(limit);
    let elapsed = start.elapsed();
    println!("ASM Counter: {} | Tempo: {:.9}s", res, elapsed.as_secs_f64());

    Ok(())
}