use desktop_ai_client_lib::telemetry::memory_replay::{default_fixture, run_replay};

fn main() {
    let report = run_replay(&default_fixture());
    println!("memory engine replay (Phase 1, shadow mode)");
    println!("cases_run: {}", report.cases_run);
    println!("precision: {:.4}", report.precision);
    println!("useful_recall: {:.4}", report.useful_recall);
    println!("contradiction_rate: {:.4}", report.contradiction_rate);
    println!("token_cost_total: {}", report.token_cost_total);
    println!("task_delta: {:.4}", report.task_delta);
}
