//! Allocation profiling for the parse/aggregate pipeline (Plan A evaluation).
//!
//! A counting global allocator reports, per adapter and per stage:
//!   - allocs / bytes allocated (cumulative churn, i.e. transient work)
//!   - peak live bytes (transient high-water, incl. DOM trees + file contents)
//!   - retained bytes (still live after the stage, i.e. the entry cache)
//! For claude/codex it additionally estimates the serde_json::Value DOM share:
//! how many lines pass the byte pre-filter, and what one Value parse costs.

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use llm_usage::core::adapter::claude::ClaudeAdapter;
use llm_usage::core::adapter::codex::CodexAdapter;
use llm_usage::core::adapter::gemini::GeminiAdapter;
use llm_usage::core::adapter::opencode::OpenCodeAdapter;
use llm_usage::core::adapter::UsageAdapter;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

struct CountingAlloc;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
struct Snap {
    allocs: usize,
    bytes: usize,
    live: usize,
    peak: usize,
}

fn snap() -> Snap {
    Snap {
        allocs: ALLOCS.load(Ordering::Relaxed),
        bytes: BYTES.load(Ordering::Relaxed),
        live: LIVE.load(Ordering::Relaxed),
        peak: PEAK.load(Ordering::Relaxed),
    }
}

fn reset_peak() {
    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
}

fn mb(b: usize) -> f64 {
    b as f64 / 1_048_576.0
}

fn collect_jsonl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_jsonl(&p, out);
        } else if p.extension().is_some_and(|x| x == "jsonl") {
            out.push(p);
        }
    }
}

/// Count lines passing the adapter's byte pre-filter, and measure the average
/// cost of one serde_json::Value DOM parse on a sample of those lines.
fn dom_estimate(files: &[PathBuf], filter: fn(&str) -> bool, label: &str) {
    let mut total_lines = 0usize;
    let mut pass_lines = 0usize;
    let mut sample: Vec<String> = Vec::new();

    for f in files {
        let Ok(content) = std::fs::read_to_string(f) else { continue };
        for line in content.lines() {
            total_lines += 1;
            if filter(line) {
                pass_lines += 1;
                if sample.len() < 300 && pass_lines % 7 == 0 {
                    sample.push(line.to_string());
                }
            }
        }
    }

    let before = snap();
    reset_peak();
    let mut parsed = 0usize;
    for line in &sample {
        if serde_json::from_str::<serde_json::Value>(line).is_ok() {
            parsed += 1;
        }
    }
    let after = snap();

    let per_line_allocs = (after.allocs - before.allocs) as f64 / parsed.max(1) as f64;
    let per_line_bytes = (after.bytes - before.bytes) as f64 / parsed.max(1) as f64;
    let est_allocs = per_line_allocs * pass_lines as f64;
    let est_mb = per_line_bytes * pass_lines as f64 / 1_048_576.0;

    println!(
        "    [{label}] lines total {:>9} | pass-filter {:>8} ({:>5.1}%) | Value DOM/line: {:>5.1} allocs, {:>7.0} B | est DOM churn: {:>9.0} allocs, {:>8.1} MB",
        total_lines,
        pass_lines,
        100.0 * pass_lines as f64 / total_lines.max(1) as f64,
        per_line_allocs,
        per_line_bytes,
        est_allocs,
        est_mb,
    );
}

fn measure(adapter: &dyn UsageAdapter, dom: Option<fn(&str) -> bool>) {
    let paths = adapter.find_data_paths().unwrap_or_default();
    let name = format!("{:?}", adapter.source());

    if let Some(filter) = dom {
        let mut files = Vec::new();
        for p in &paths {
            collect_jsonl(p, &mut files);
        }
        let label = name.clone();
        dom_estimate(&files, filter, &label);
    }

    let before = snap();
    reset_peak();
    let t0 = Instant::now();
    let entries = adapter.load_entries(&paths).unwrap_or_default();
    let load_ms = t0.elapsed().as_secs_f64() * 1e3;
    let after_load = snap();

    let t1 = Instant::now();
    let daily = adapter.aggregate_daily(&entries);
    let monthly = adapter.aggregate_monthly(&entries);
    let session = adapter.aggregate_session(&entries);
    let blocks = adapter.aggregate_blocks(&entries, 5);
    let agg_ms = t1.elapsed().as_secs_f64() * 1e3;
    let after_agg = snap();

    let n = entries.len().max(1) as f64;
    println!(
        "    [{name}] LOAD  {:>8.1}ms | churn {:>10} allocs {:>9.1} MB | peak live {:>8.1} MB | retained {:>7.1} MB | {:>6.0} allocs/entry",
        load_ms,
        after_load.allocs - before.allocs,
        mb(after_load.bytes - before.bytes),
        mb(after_load.peak.saturating_sub(before.live)),
        mb(after_load.live.saturating_sub(before.live)),
        (after_load.allocs - before.allocs) as f64 / n,
    );
    println!(
        "    [{name}] AGG   {:>8.1}ms | churn {:>10} allocs {:>9.1} MB | retained {:>7.1} MB | reports {}/{}/{}/{}",
        agg_ms,
        after_agg.allocs - after_load.allocs,
        mb(after_agg.bytes - after_load.bytes),
        mb(after_agg.live.saturating_sub(after_load.live)),
        daily.len(),
        monthly.len(),
        session.len(),
        blocks.len(),
    );
    drop(entries);
}

fn main() {
    println!("\nAllocation profile per adapter (counting global allocator)\n");
    measure(&CodexAdapter::new(), Some(|l: &str| {
        l.contains("token_count") || l.contains("turn_context")
    }));
    measure(&ClaudeAdapter::new(), Some(|l: &str| {
        l.contains("\"usage\"") || l.contains("\"cwd\"")
    }));
    measure(&GeminiAdapter::new(), None);
    measure(&OpenCodeAdapter::new(), None);
    println!();
}
