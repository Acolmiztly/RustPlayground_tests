#[inline(always)]
pub fn count_ultra_fast_asm(limit: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx512f") {
            return count_avx512_real(limit);
        } else if std::arch::is_x86_feature_detected!("avx2") {
            return count_avx2_real(limit);
        } else if std::arch::is_x86_feature_detected!("sse2") {
            return count_sse2_real(limit);
        } else {
            return count_scalar_fallback(limit);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return count_neon_real(limit);
        } else {
            return count_scalar_fallback(limit);
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        count_scalar_fallback(limit)
    }
}

// AVX-512: 4 accumulatori ZMM (512-bit) = 64 addizioni parallele per iterazione
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn count_avx512_real(limit: u32) -> u32 {
    let cnt = limit / 64;
    let rem = limit % 64;
    let mut sum: u32;

    unsafe {
        core::arch::asm!(
        // Zeroing dei registri (Sintassi Intel: op dest, src1, src2)
        "vpxord zmm0, zmm0, zmm0",
        "vpxord zmm2, zmm2, zmm2",
        "vpxord zmm3, zmm3, zmm3",
        "vpxord zmm4, zmm4, zmm4",

        // Creazione vettore di 1s
        "vmovd xmm5, {one:e}",
        "vpbroadcastd zmm5, xmm5",     // zmm5 = [1,1,...,1] (16 elementi)

        "test {cnt:e}, {cnt:e}",
        "jz 3f",
        "2:",
        // 4 addizioni vettoriali indipendenti
        "vpaddd zmm0, zmm0, zmm5",
        "vpaddd zmm2, zmm2, zmm5",
        "vpaddd zmm3, zmm3, zmm5",
        "vpaddd zmm4, zmm4, zmm5",
        "dec {cnt:e}",
        "jnz 2b",

        "3:",
        // Somma i 4 accumulatori
        "vpaddd zmm0, zmm0, zmm2",
        "vpaddd zmm0, zmm0, zmm3",
        "vpaddd zmm0, zmm0, zmm4",

        // Riduzione orizzontale ZMM -> YMM -> XMM -> Scalar
        "vextracti64x4 ymm1, zmm0, 1",
        "vpaddd ymm0, ymm0, ymm1",
        "vextracti128 xmm1, ymm0, 1",
        "vpaddd xmm0, xmm0, xmm1",
        "vphaddd xmm0, xmm0, xmm0",
        "vphaddd xmm0, xmm0, xmm0",
        "vmovd {sum:e}, xmm0",

        "add {sum:e}, {rem:e}",

        cnt = inout(reg) cnt => _,
        rem = inout(reg) rem => _,
        sum = out(reg) sum,
        one = in(reg) 1u32,
        // CLOBBER: Fondamentale dire al compilatore che stiamo sporcando questi registri
        out("zmm0") _, out("zmm2") _, out("zmm3") _, out("zmm4") _, out("zmm5") _,
        out("ymm1") _,
        options(nostack, nomem)
        );
    }
    sum
}

// AVX2: 4 accumulatori YMM (256-bit) = 32 addizioni parallele per iterazione
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn count_avx2_real(limit: u32) -> u32 {
    let cnt = limit / 32;
    let rem = limit % 32;
    let mut sum: u32;

    unsafe {
        core::arch::asm!(
        "vpxor ymm0, ymm0, ymm0",
        "vpxor ymm2, ymm2, ymm2",
        "vpxor ymm3, ymm3, ymm3",
        "vpxor ymm4, ymm4, ymm4",
        "vmovd xmm5, {one:e}",
        "vpbroadcastd ymm5, xmm5",

        "test {cnt:e}, {cnt:e}",
        "jz 3f",
        "2:",
        "vpaddd ymm0, ymm0, ymm5",
        "vpaddd ymm2, ymm2, ymm5",
        "vpaddd ymm3, ymm3, ymm5",
        "vpaddd ymm4, ymm4, ymm5",
        "dec {cnt:e}",
        "jnz 2b",

        "3:",
        "vpaddd ymm0, ymm0, ymm2",
        "vpaddd ymm0, ymm0, ymm3",
        "vpaddd ymm0, ymm0, ymm4",

        "vextracti128 xmm1, ymm0, 1",
        "vpaddd xmm0, xmm0, xmm1",
        "vphaddd xmm0, xmm0, xmm0",
        "vphaddd xmm0, xmm0, xmm0",
        "vmovd {sum:e}, xmm0",

        "add {sum:e}, {rem:e}",

        "vzeroupper",  // Evita penalty SSE legacy

        cnt = inout(reg) cnt => _,
        rem = inout(reg) rem => _,
        sum = out(reg) sum,
        one = in(reg) 1u32,
        out("ymm0") _, out("ymm2") _, out("ymm3") _, out("ymm4") _, out("ymm5") _,
        out("xmm1") _,
        options(nostack, nomem)
        );
    }
    sum
}

// SSE2: 4 accumulatori XMM (128-bit) = 16 addizioni parallele per iterazione
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn count_sse2_real(limit: u32) -> u32 {
    let cnt = limit / 16;
    let rem = limit % 16;
    let mut sum: u32;

    unsafe {
        core::arch::asm!(
        "pxor xmm0, xmm0",
        "pxor xmm2, xmm2",
        "pxor xmm3, xmm3",
        "pxor xmm4, xmm4",
        "movd xmm5, {one:e}",
        "pshufd xmm5, xmm5, 0",

        "test {cnt:e}, {cnt:e}",
        "jz 3f",
        "2:",
        "paddd xmm0, xmm5",
        "paddd xmm2, xmm5",
        "paddd xmm3, xmm5",
        "paddd xmm4, xmm5",
        "dec {cnt:e}",
        "jnz 2b",

        "3:",
        "paddd xmm0, xmm2",
        "paddd xmm0, xmm3",
        "paddd xmm0, xmm4",

        // Riduzione orizzontale XMM sicura
        "pshufd xmm1, xmm0, 0x0E",
        "paddd xmm0, xmm1",
        "pshufd xmm1, xmm0, 0x01",
        "paddd xmm0, xmm1",
        "movd {sum:e}, xmm0",

        "add {sum:e}, {rem:e}",

        cnt = inout(reg) cnt => _,
        rem = inout(reg) rem => _,
        sum = out(reg) sum,
        one = in(reg) 1u32,
        out("xmm0") _, out("xmm2") _, out("xmm3") _, out("xmm4") _, out("xmm5") _,
        out("xmm1") _,
        options(nostack, nomem)
        );
    }
    sum
}

// ARM NEON: 4 accumulatori Q (128-bit) = 16 addizioni parallele per iterazione
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn count_neon_real(limit: u32) -> u32 {
    let cnt = limit / 16;
    let rem = limit % 16;
    let mut sum: u32;

    unsafe {
        core::arch::asm!(
        "movi v0.4s, #0",
        "movi v1.4s, #0",
        "movi v2.4s, #0",
        "movi v3.4s, #0",
        "movi v4.4s, #1",

        "cbz {cnt:w}, 2f",
        "1:",
        "add v0.4s, v0.4s, v4.4s",
        "add v1.4s, v1.4s, v4.4s",
        "add v2.4s, v2.4s, v4.4s",
        "add v3.4s, v3.4s, v4.4s",
        "subs {cnt:w}, {cnt:w}, #1",
        "b.ne 1b",

        "2:",
        "add v0.4s, v0.4s, v1.4s",
        "add v0.4s, v0.4s, v2.4s",
        "add v0.4s, v0.4s, v3.4s",

        // Riduzione orizzontale ARM sicura tramite addp
        "addp v0.4s, v0.4s, v0.4s",
        "addp v0.4s, v0.4s, v0.4s",
        "mov {sum:w}, v0.s[0]",

        "add {sum:w}, {sum:w}, {rem:w}",

        cnt = inout(reg) cnt => _,
        rem = inout(reg) rem => _,
        sum = out(reg) sum,
        out("v0") _, out("v1") _, out("v2") _, out("v3") _, out("v4") _,
        options(nostack, nomem)
        );
    }
    sum
}

#[inline(always)]
fn count_scalar_fallback(limit: u32) -> u32 {
    limit
}

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limit = 1_000_000_000_u32;

    // Warmup per stabilizzare frequenze e cache della CPU
    for _ in 0..10 {
        std::hint::black_box(count_ultra_fast_asm(limit));
    }

    let start = std::time::Instant::now();
    let iterations = 1;
    let mut total = 0u32;
    for _ in 0..iterations {
        total = total.wrapping_add(count_ultra_fast_asm(limit));
    }
    let elapsed = start.elapsed();

    std::hint::black_box(total);

    println!("Max Performance ASM: {} | {:.9}s | {:.6}M iter/s",
             total, elapsed.as_secs_f64(),
             (limit as f64 * iterations as f64) / elapsed.as_secs_f64() / 1_000_000.0);
    Ok(())
}