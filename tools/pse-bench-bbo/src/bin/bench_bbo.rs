//! Black-box optimization benchmark — TRITON spiral vs random search vs Halton.
//!
//! Runs each optimizer on each test function for the same fixed
//! evaluation budget. Reports a markdown table to stdout with
//! per-(function, optimizer) simple regret and cumulative regret.

use pse_bench_bbo::{
    functions::{Ackley, DriftRastrigin, Rastrigin, Sphere},
    optimizers::{HaltonSearch, RandomSearch, TritonSpiralBBO},
    run_optimization, Optimizer, TestFunction, Trajectory,
};

const BUDGET: usize = 200;
const SEED: u64 = 0x1234_5678_ABCD_EF01;

fn main() {
    println!("# PSE BBO Benchmark");
    println!();
    println!(
        "Budget: **{}** evaluations per (function, optimizer) pair. Seed: \
         {:#x}. All optimizers operate on `[0, 1]^2`. Lower regret = better.",
        BUDGET, SEED
    );
    println!();

    let functions: Vec<Box<dyn TestFunction>> = vec![
        Box::new(Sphere::default()),
        Box::new(Rastrigin),
        Box::new(Ackley),
        Box::new(DriftRastrigin::default()),
    ];

    for f in &functions {
        println!("## Function: `{}`", f.name());
        println!();
        println!("| Optimizer | Final simple regret | Cumulative regret | Best value | Best point |");
        println!("|-----------|---------------------|-------------------|------------|------------|");
        let trajectories = vec![
            run_with_random(f.as_ref()),
            run_with_halton(f.as_ref()),
            run_with_triton(f.as_ref()),
        ];
        for traj in &trajectories {
            println!(
                "| `{}` | {:.6} | {:.4} | {:.6} | ({:.4}, {:.4}) |",
                traj.optimizer,
                traj.final_simple_regret,
                traj.final_cumulative_regret,
                traj.best_value,
                traj.best_point.first().copied().unwrap_or(f64::NAN),
                traj.best_point.get(1).copied().unwrap_or(f64::NAN),
            );
        }
        println!();
    }

    println!("## Interpretation");
    println!();
    println!(
        "- **`random_search`** is the worst-case-of-don't-be-clever floor — \
         any adaptive method should beat it on multi-modal landscapes."
    );
    println!(
        "- **`halton`** is a strong non-adaptive baseline (deterministic \
         low-discrepancy coverage). It has no idea about function values \
         but guarantees that within the budget, the unit square is \
         systematically sampled."
    );
    println!(
        "- **`triton_spiral`** is the golden-angle spiral component of \
         PSE's TRITON navigator, isolated from its adaptive-mesh \
         machinery. The pure spiral is also non-adaptive but has a \
         different aperiodic-coverage profile from Halton."
    );
    println!();
    println!(
        "On stationary functions all three converge to similar simple \
         regret given enough budget. The interesting comparison is the \
         **drift_rastrigin** row: a non-stationary landscape where the \
         optimum precesses during the run — none of the three baselines \
         here can track drift, so the row reports the *baseline ceiling* \
         that an adaptive optimizer (the full mesh + Fiedler-momentum \
         machinery exposed in `pse_core::explore`) would need to beat."
    );
}

fn run_with_random(f: &dyn TestFunction) -> Trajectory {
    let mut opt = RandomSearch::new(SEED, 2);
    run_optimization(&mut opt, f, BUDGET)
}

fn run_with_halton(f: &dyn TestFunction) -> Trajectory {
    let mut opt = HaltonSearch::new(1, 2);
    run_optimization(&mut opt, f, BUDGET)
}

fn run_with_triton(f: &dyn TestFunction) -> Trajectory {
    let mut opt = TritonSpiralBBO::new(SEED);
    run_optimization(&mut opt, f, BUDGET)
}
