use std::path::{Path, PathBuf};
use std::time::Instant;

use llm_usage::core::adapter::claude::ClaudeAdapter;
use llm_usage::core::adapter::codex::CodexAdapter;
use llm_usage::core::adapter::gemini::GeminiAdapter;
use llm_usage::core::adapter::opencode::OpenCodeAdapter;
use llm_usage::core::adapter::UsageAdapter;

fn dir_stats(paths: &[PathBuf]) -> (u64, u64) {
    fn walk(p: &Path, files: &mut u64, bytes: &mut u64) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    walk(&path, files, bytes);
                } else if let Ok(m) = e.metadata() {
                    *files += 1;
                    *bytes += m.len();
                }
            }
        }
    }
    let (mut files, mut bytes) = (0, 0);
    for p in paths {
        walk(p, &mut files, &mut bytes);
    }
    (files, bytes)
}

fn measure(adapter: &dyn UsageAdapter) {
    let t0 = Instant::now();
    let paths = adapter.find_data_paths().unwrap_or_default();
    let find_ms = t0.elapsed().as_secs_f64() * 1e3;

    let (files, bytes) = dir_stats(&paths);

    let t1 = Instant::now();
    let entries = adapter.load_entries(&paths).unwrap_or_default();
    let load_ms = t1.elapsed().as_secs_f64() * 1e3;

    let t2 = Instant::now();
    let daily = adapter.aggregate_daily(&entries);
    let monthly = adapter.aggregate_monthly(&entries);
    let session = adapter.aggregate_session(&entries);
    let blocks = adapter.aggregate_blocks(&entries, 5);
    let agg_ms = t2.elapsed().as_secs_f64() * 1e3;

    println!(
        "{:9} | find {:7.1}ms | load {:9.1}ms | agg {:6.1}ms | files {:5} | data {:8.1} MB | entries {:7} | reports d/m/s/b {}/{}/{}/{}",
        format!("{:?}", adapter.source()),
        find_ms,
        load_ms,
        agg_ms,
        files,
        bytes as f64 / 1048576.0,
        entries.len(),
        daily.len(),
        monthly.len(),
        session.len(),
        blocks.len(),
    );
}

fn main() {
    println!("Measuring adapter load times (cold-ish, no app cache)\n");
    let total = Instant::now();
    measure(&ClaudeAdapter::new());
    measure(&CodexAdapter::new());
    measure(&GeminiAdapter::new());
    measure(&OpenCodeAdapter::new());
    println!("\nSequential total: {:.1}ms", total.elapsed().as_secs_f64() * 1e3);
    println!("Note: refresh runs adapters sequentially in preload(); per-file JSON parsing dominates.");
}
