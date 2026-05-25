mod benchmarks;
mod baselines;
mod datasets;
mod fixtures;
mod manifest;
mod metrics;
mod report;
mod stats;

use std::path::PathBuf;

fn main() {
    let out_dir = parse_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    println!("PSE Paper Benchmark Suite");
    println!("═════════════════════════════════════════════");
    println!("  Output : {}", out_dir.display());
    println!();

    let results = benchmarks::run_all(&out_dir);

    let report_md = report::markdown::render(&results);
    let tables_tex = report::latex::render_tables(&results);

    std::fs::write(out_dir.join("report.md"), &report_md).expect("write report.md");
    std::fs::write(out_dir.join("tables.tex"), &tables_tex).expect("write tables.tex");

    let mf = manifest::build(&out_dir, &results);
    let mf_json = serde_json::to_string_pretty(&mf).expect("serialize manifest");
    std::fs::write(out_dir.join("manifest.json"), &mf_json).expect("write manifest.json");

    report::zenodo::write_zenodo_dir(&out_dir, &results, &mf);

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Done.");
    println!("  manifest.json : {}", mf.self_hash);
    println!("  Upload zenodo/ to https://zenodo.org/deposit");
}

fn parse_out_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--out" || args[i] == "--out-dir") && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
        i += 1;
    }
    PathBuf::from("./bench-output")
}
