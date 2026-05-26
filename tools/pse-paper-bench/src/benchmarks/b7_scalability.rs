use crate::fixtures;
use pse_adapter_il::text_to_vector8;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize, Clone, Default)]
pub struct LatencyPoint {
    pub n_crystals: usize,
    pub mean_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct B7Results {
    pub points: Vec<LatencyPoint>,
    pub n_retrieval_calls_per_point: usize,
}

const SIZES: &[usize] = &[10, 100, 1_000, 5_000];
const N_CALLS: usize = 100;

pub fn run() -> B7Results {
    let query_vec = text_to_vector8("epistemic stability convergence");
    let mut points = Vec::new();

    for &n in SIZES {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut store = fixtures::open_store(dir.path());
        for i in 0..n {
            let stab = 0.5 + 0.4 * ((i as f64 / n as f64) * std::f64::consts::PI).sin().abs();
            let kura = 0.4 + 0.3 * ((i as f64 * 1.618) % 1.0);
            let crystal = fixtures::build_crystal(&format!("b7-{n}-{i}"), stab, kura);
            let _ = fixtures::commit_crystal(
                &mut store,
                &crystal,
                &format!("synthetic document {i} for scalability benchmark"),
                i + 1,
            );
        }
        let mut latencies_us: Vec<f64> = Vec::with_capacity(N_CALLS);
        for _ in 0..N_CALLS {
            let t = Instant::now();
            let _ = store.score_tripolar(&query_vec);
            latencies_us.push(t.elapsed().as_secs_f64() * 1e6);
        }
        latencies_us.sort_by(f64::total_cmp);
        let mean = latencies_us.iter().sum::<f64>() / latencies_us.len() as f64;
        let p95 = latencies_us[(latencies_us.len() as f64 * 0.95) as usize];
        let p99 =
            latencies_us[((latencies_us.len() as f64 * 0.99) as usize).min(latencies_us.len() - 1)];
        println!("    N={n}: mean={mean:.1}µs  p95={p95:.1}µs  p99={p99:.1}µs");
        points.push(LatencyPoint {
            n_crystals: n,
            mean_us: mean,
            p95_us: p95,
            p99_us: p99,
        });
    }
    B7Results {
        points,
        n_retrieval_calls_per_point: N_CALLS,
    }
}
