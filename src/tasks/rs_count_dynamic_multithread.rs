use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

const MT_THRESHOLD: u32 = 1_000_000;
const SPIN_ITERATIONS: u32 = 10000;
const WORK_UNIT_SIZE: u32 = 10_000_000; // 10M iterazioni per work unit

// ==================== WORK QUEUE CONDIVISA ====================
struct WorkQueue {
    remaining: AtomicU32,      // Iterazioni totali rimanenti
    completed: AtomicU64,      // Iterazioni completate totali
}

// ==================== WORKER SLOT ====================
struct WorkerSlot {
    result: AtomicU64,         // Risultato parziale del worker
    work_batch: AtomicU64,     // Batch ID corrente
    done_batch: AtomicU64,     // Batch ID completato
}

// ==================== THREAD POOL CON WORK STEALING ====================
struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    slots: Vec<Arc<WorkerSlot>>,
    work_queue: Arc<WorkQueue>,
    batch_counter: AtomicU64,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let mut workers = Vec::with_capacity(size);
        let mut slots = Vec::with_capacity(size);
        let work_queue = Arc::new(WorkQueue {
            remaining: AtomicU32::new(0),
            completed: AtomicU64::new(0),
        });

        for _ in 0..size {
            let slot = Arc::new(WorkerSlot {
                result: AtomicU64::new(0),
                work_batch: AtomicU64::new(0),
                done_batch: AtomicU64::new(0),
            });
            slots.push(Arc::clone(&slot));

            let worker_slot = Arc::clone(&slot);
            let worker_queue = Arc::clone(&work_queue);

            let handle = thread::spawn(move || {
                let mut local_batch_id = 0u64;

                loop {
                    // Attendi nuovo batch
                    let mut spin_count = 0u32;
                    loop {
                        let current_batch = worker_slot.work_batch.load(Ordering::Acquire);
                        if current_batch != local_batch_id {
                            break;
                        }

                        spin_count += 1;
                        if spin_count < SPIN_ITERATIONS {
                            core::hint::spin_loop();
                        } else {
                            thread::yield_now();
                        }
                    }

                    local_batch_id = worker_slot.work_batch.load(Ordering::Acquire);

                    // Controlla segnale di terminazione
                    if local_batch_id == u64::MAX {
                        break;
                    }

                    // Reset risultato locale
                    let mut local_result = 0u64;

                    // Work stealing: prendi work units finché ce ne sono
                    loop {
                        // Prendi un work unit atomicamente
                        let current = worker_queue.remaining.load(Ordering::Relaxed);
                        if current == 0 {
                            break; // Non c'è più lavoro
                        }

                        let chunk = current.min(WORK_UNIT_SIZE);

                        // Tenta di decrementare atomicamente
                        match worker_queue.remaining.compare_exchange(
                            current,
                            current.saturating_sub(chunk),
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                // Esegui il lavoro
                                let result = count_ultra_fast_asm(chunk);
                                local_result += result as u64;
                            }
                            Err(_) => continue, // Qualcun altro ha preso il lavoro, riprova
                        }
                    }

                    // Salva risultato e segna completamento
                    worker_slot.result.store(local_result, Ordering::Relaxed);
                    worker_slot.done_batch.store(local_batch_id, Ordering::Release);
                }
            });

            workers.push(handle);
        }

        ThreadPool {
            workers,
            slots,
            work_queue,
            batch_counter: AtomicU64::new(0),
        }
    }

    #[inline(never)]
    fn execute(&self, total_limit: u32) -> u32 {
        let batch_id = self.batch_counter.fetch_add(1, Ordering::SeqCst) + 1;

        // Reset work queue
        self.work_queue.remaining.store(total_limit, Ordering::SeqCst);
        self.work_queue.completed.store(0, Ordering::SeqCst);

        // Sveglia tutti i worker
        for slot in &self.slots {
            slot.work_batch.store(batch_id, Ordering::Release);
        }

        // Attendi completamento di tutti i worker
        for slot in &self.slots {
            let mut spin_count = 0u32;
            loop {
                if slot.done_batch.load(Ordering::Acquire) == batch_id {
                    break;
                }

                spin_count += 1;
                if spin_count < SPIN_ITERATIONS {
                    core::hint::spin_loop();
                } else {
                    thread::yield_now();
                }
            }
        }

        // Somma i risultati
        let total: u64 = self.slots.iter()
            .map(|slot| slot.result.load(Ordering::Relaxed))
            .sum();

        total as u32
    }

    fn size(&self) -> usize {
        self.slots.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Segnala terminazione
        for slot in &self.slots {
            slot.work_batch.store(u64::MAX, Ordering::Release);
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

// ==================== RESTO DEL CODICE ====================
#[inline(never)]
pub fn count_multithreaded_fast(limit: u32, pool: &ThreadPool) -> u32 {
    let num_threads = pool.size();

    if limit < MT_THRESHOLD || num_threads <= 1 {
        return count_ultra_fast_asm(limit);
    }

    pool.execute(limit)
}

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

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limit = 1_000_000_000_u32;
    let iterations = 1;

    let num_cores = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let physical_cores = (num_cores / 2).max(1);
    let thread_count = if num_cores > 8 { physical_cores } else { num_cores };

    println!("🖥️  Core logici rilevati: {}", num_cores);
    println!("🎯 Core fisici stimati: {} (uso {} thread)", physical_cores, thread_count);
    println!("📊 Test con limit = {:>12} e {} iterazioni", limit, iterations);
    println!("   Totale lavoro: {} iterazioni", (limit as u64) * (iterations as u64));

    println!("\n🔧 Inizializzazione Thread Pool con Work Stealing...");
    let pool = ThreadPool::new(thread_count);

    println!("🔥 Warmup...");
    for _ in 0..5 {
        std::hint::black_box(count_ultra_fast_asm(limit / 10));
        std::hint::black_box(count_multithreaded_fast(limit / 10, &pool));
    }

    println!("\n⏱️  Benchmark Single Thread...");
    let start = std::time::Instant::now();
    let mut acc_st = 0u64;
    for _ in 0..iterations {
        acc_st = acc_st.wrapping_add(count_ultra_fast_asm(limit) as u64);
    }
    let elapsed_st = start.elapsed();
    std::hint::black_box(acc_st);

    let total_work = (limit as u64) * (iterations as u64);
    let st_speed = (total_work as f64) / elapsed_st.as_secs_f64() / 1_000_000_000.0;
    println!("   Single Thread: {:.6}s | {:.2} Giter/s", elapsed_st.as_secs_f64(), st_speed);

    println!("\n⏱️  Benchmark Multi Thread Work Stealing ({} thread)...", thread_count);
    let start = std::time::Instant::now();
    let mut acc_mt = 0u64;
    for _ in 0..iterations {
        acc_mt = acc_mt.wrapping_add(count_multithreaded_fast(limit, &pool) as u64);
    }
    let elapsed_mt = start.elapsed();
    std::hint::black_box(acc_mt);

    let mt_speed = (total_work as f64) / elapsed_mt.as_secs_f64() / 1_000_000_000.0;
    println!("   Multi Thread:  {:.6}s | {:.2} Giter/s", elapsed_mt.as_secs_f64(), mt_speed);

    let speedup = elapsed_st.as_secs_f64() / elapsed_mt.as_secs_f64();
    let efficiency = (speedup / thread_count as f64) * 100.0;

    println!("\n🚀 Speedup: {:.2}x su {} thread (efficienza: {:.1}%)",
             speedup, thread_count, efficiency);
    println!("📈 Throughput totale: {:.2} Giter/s", mt_speed);

    println!("\n✅ Verifica correttezza...");
    let expected = limit;
    let st_result = count_ultra_fast_asm(limit);
    let mt_result = count_multithreaded_fast(limit, &pool);

    assert_eq!(st_result, expected, "Single thread mismatch!");
    assert_eq!(mt_result, expected, "Multi thread mismatch!");
    println!("   Single Thread: {} ✓", st_result);
    println!("   Multi Thread:  {} ✓", mt_result);

    Ok(())
}

// ==================== IMPLEMENTAZIONI SIMD (invariate) ====================
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn count_avx512_real(limit: u32) -> u32 {
    let cnt = limit / 64;
    let rem = limit % 64;
    let mut sum: u32;
    unsafe {
        core::arch::asm!(
        "vpxord zmm0, zmm0, zmm0",
        "vpxord zmm2, zmm2, zmm2",
        "vpxord zmm3, zmm3, zmm3",
        "vpxord zmm4, zmm4, zmm4",
        "vmovd xmm5, {one:e}",
        "vpbroadcastd zmm5, xmm5",
        "test {cnt:e}, {cnt:e}",
        "jz 3f",
        "2:",
        "vpaddd zmm0, zmm0, zmm5",
        "vpaddd zmm2, zmm2, zmm5",
        "vpaddd zmm3, zmm3, zmm5",
        "vpaddd zmm4, zmm4, zmm5",
        "dec {cnt:e}",
        "jnz 2b",
        "3:",
        "vpaddd zmm0, zmm0, zmm2",
        "vpaddd zmm0, zmm0, zmm3",
        "vpaddd zmm0, zmm0, zmm4",
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
        out("zmm0") _, out("zmm2") _, out("zmm3") _, out("zmm4") _, out("zmm5") _,
        out("ymm1") _,
        options(nostack, nomem)
        );
    }
    sum
}

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
        "vzeroupper",
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