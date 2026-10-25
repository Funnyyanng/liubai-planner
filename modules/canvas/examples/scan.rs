//! Read-only integration probe. Uses the existing Keychain credential.
fn main() {
    let rows = planner_canvas::scan(
        &planner_framework::external::CanvasReader::connect().expect("credentials"),
        planner_framework::time::now(),
    )
    .expect("scan");
    println!(
        "courses={} future_courses={} assignments={} events={} undated={} failed={}",
        rows.len(),
        rows.iter()
            .filter(|p| !p.assignments.is_empty() || !p.events.is_empty())
            .count(),
        rows.iter().map(|p| p.assignments.len()).sum::<usize>(),
        rows.iter().map(|p| p.events.len()).sum::<usize>(),
        rows.iter().map(|p| p.undated.len()).sum::<usize>(),
        rows.iter()
            .filter(|p| p.warnings.iter().any(|w| w.starts_with("未能检查")))
            .count()
    );
}
