/// This code manage the execution of tasks in this directory
/// For each task add "pub mod [task_name]" + add to enum (TaskId) + add to (matct self) in TaskId{} + add to (fn execute):
/// The task code must use this as entry point:
///
/// pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///                 ---> YOUR CODE <---
/// --> end with  Ok(())
/// }

pub mod rs_count_std;
pub mod rs_count_asm;
pub mod rs_count_parallel_asm;
pub mod rs_count_parallel_max_asm;
pub mod rs_count_parallel_dynamic_beta;
pub mod rs_count_dynamic_multithread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskId {
    RsCountStd,
    RsCountAsm,
    RsCountParallelAsm,
    RsCountParallelMax,
    RsCountParallelDynamicBeta,
    RsCountParallelDynamicMultithread,

}

impl TaskId {
    /// Compile-time static register. Verify the coherence
    pub const ALL: &'static [Self] = &[
        Self::RsCountStd,
        Self::RsCountAsm,
        Self::RsCountParallelAsm,
        Self::RsCountParallelMax,
        Self::RsCountParallelDynamicBeta,
        Self::RsCountParallelDynamicMultithread,
    ];

    /// Explicit mapping
    pub const fn name(&self) -> &'static str {
        match self {
            Self::RsCountStd => "RsCountStd",
            Self::RsCountAsm => "RsCountAsm",
            Self::RsCountParallelAsm => "RsCountParallelAsm",
            Self::RsCountParallelMax => "RsCountParallelMax",
            Self::RsCountParallelDynamicBeta => "RsCountParallelDynamicBeta",
            Self::RsCountParallelDynamicMultithread => "RsCountParallelDynamicMultithread",

        }
    }
}

impl std::str::FromStr for TaskId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Search in the static register: zero-alloc, optimized with lookup table
        TaskId::ALL
            .iter()
            .find(|task| task.name() == s)
            .copied()
            .ok_or("Task non trovato nel registro")
    }
}

/// Dispatch zero-cost: match compiles in a  jump table or in binary search.
pub fn execute(id: TaskId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match id {
        TaskId::RsCountStd => rs_count_std::run(),
        TaskId::RsCountAsm => rs_count_asm::run(),
        TaskId::RsCountParallelAsm => rs_count_parallel_asm::run(),
        TaskId::RsCountParallelMax => rs_count_parallel_max_asm::run(),
        TaskId::RsCountParallelDynamicBeta => rs_count_parallel_dynamic_beta::run(),
        TaskId::RsCountParallelDynamicMultithread => rs_count_dynamic_multithread::run(),
    }
}