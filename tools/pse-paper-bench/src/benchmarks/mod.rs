pub mod b1_retrieval;
pub mod b2_noise;
pub mod b3_constitutional;
pub mod b4_etv;
pub mod b5_anomaly;
pub mod b6_agent;
pub mod b7_scalability;

use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone, Default)]
pub struct AllResults {
    pub b1: b1_retrieval::B1Results,
    pub b2: b2_noise::B2Results,
    pub b3: b3_constitutional::B3Results,
    pub b4: b4_etv::B4Results,
    pub b5: b5_anomaly::B5Results,
    pub b6: b6_agent::B6Results,
    pub b7: b7_scalability::B7Results,
}

pub fn run_all(out_dir: &Path) -> AllResults {
    println!("[B1] Retrieval quality (Pfauenthron++ vs BM25 vs cosine-only vs stability-only)...");
    let b1 = b1_retrieval::run();
    write_json(out_dir, "b1_retrieval.json", &b1);

    println!("[B2] Retrieval robustness under noise...");
    let b2 = b2_noise::run();
    write_json(out_dir, "b2_noise.json", &b2);

    println!("[B3] Constitutional precision/recall...");
    let b3 = b3_constitutional::run();
    write_json(out_dir, "b3_constitutional.json", &b3);

    println!("[B4] ETV reasoning chain coherence...");
    let b4 = b4_etv::run();
    write_json(out_dir, "b4_etv.json", &b4);

    println!("[B5] Anomaly detection (wrapping pse-bench-gt)...");
    let b5 = b5_anomaly::run();
    write_json(out_dir, "b5_anomaly.json", &b5);

    println!("[B6] Agent relevance ranking (wrapping pse-eval-matrix)...");
    let b6 = b6_agent::run();
    write_json(out_dir, "b6_agent.json", &b6);

    println!("[B7] Scalability (latency sweep 10/100/1K/5K crystals)...");
    let b7 = b7_scalability::run();
    write_json(out_dir, "b7_scalability.json", &b7);

    AllResults { b1, b2, b3, b4, b5, b6, b7 }
}

fn write_json<T: Serialize>(out_dir: &Path, name: &str, val: &T) {
    let json = serde_json::to_string_pretty(val).expect("serialize");
    std::fs::write(out_dir.join(name), json).expect("write json");
}
