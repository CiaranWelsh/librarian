//! Statistical Gate-2 regression detector for `librarian add` (L-068).
//!
//! Given historical `HealthReport` runs and a new measurement taken immediately
//! after a commit, decides whether retrieval regressed. Higher hit_rate / mrr
//! is better; higher fragment_rate is worse.

use crate::commands::health::HealthReport;

/// Default sensitivity: flag a metric that drops more than this many standard
/// deviations in the worsening direction relative to the history.
pub const DEFAULT_K_SIGMA: f64 = 2.0;

/// Absolute floor so sub-noise moves never trip the gate when the history is
/// flat (stdev ~ 0). A metric must worsen by MORE than both k*stdev AND this.
const MIN_MARGIN: f64 = 0.01;

/// Decide whether `after` is a retrieval regression vs the `history` distribution.
///
/// Returns `None` when there is no regression, or when history is too short to
/// establish a baseline (fewer than 2 runs). Returns `Some(reason)` naming the
/// first offending metric with its after-value and the historical mean.
///
/// Gated metrics:
///   - `hit_rate`: lower is worse
///   - `mrr`: lower is worse
///   - `fragment_rate`: higher is worse
///
/// `mean_top_score` is a drift signal and is intentionally excluded.
pub fn is_regression(
    history: &[HealthReport],
    after: &HealthReport,
    k_sigma: f64,
) -> Option<String> {
    if history.len() < 2 {
        return None;
    }

    let hit_rates: Vec<f64> = history.iter().map(|r| r.hit_rate as f64).collect();
    if let Some(msg) = check_metric(
        &hit_rates,
        after.hit_rate as f64,
        "hit-rate@k",
        k_sigma,
        true,
    ) {
        return Some(msg);
    }

    let mrrs: Vec<f64> = history.iter().map(|r| r.mrr as f64).collect();
    if let Some(msg) = check_metric(&mrrs, after.mrr as f64, "mrr", k_sigma, true) {
        return Some(msg);
    }

    let frags: Vec<f64> = history.iter().map(|r| r.fragment_rate as f64).collect();
    if let Some(msg) = check_metric(
        &frags,
        after.fragment_rate as f64,
        "fragment-rate",
        k_sigma,
        false,
    ) {
        return Some(msg);
    }

    None
}

/// Flag a single metric against its history. `lower_is_better` true means a drop
/// below the baseline is the regression (hit-rate, mrr); false means a rise above
/// it is (fragment-rate). The margin floors at `MIN_MARGIN` so a flat history
/// cannot false-positive on a sub-noise move.
fn check_metric(
    values: &[f64],
    after_val: f64,
    label: &str,
    k_sigma: f64,
    lower_is_better: bool,
) -> Option<String> {
    let m = mean(values);
    let margin = (k_sigma * stdev(values, m)).max(MIN_MARGIN);
    let (regressed, direction) = if lower_is_better {
        (after_val < m - margin, "drop")
    } else {
        (after_val > m + margin, "rise")
    };
    if regressed {
        Some(format!(
            "{label} regressed: {after_val:.3} vs baseline mean {m:.3} (>{k_sigma:.1}sigma {direction})"
        ))
    } else {
        None
    }
}

fn mean(values: &[f64]) -> f64 {
    debug_assert!(!values.is_empty(), "mean requires at least one value");
    values.iter().sum::<f64>() / values.len() as f64
}

// Population stdev: sqrt of the mean of squared deviations.
fn stdev(values: &[f64], m: f64) -> f64 {
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rep(hit: f32, mrr: f32, frag: f32) -> HealthReport {
        HealthReport {
            n: 50,
            k: 10,
            hit_rate: hit,
            mrr,
            fragment_rate: frag,
            mean_top_score: 0.5,
        }
    }

    #[test]
    fn regression_when_hitrate_drops_beyond_sigma() {
        let history = vec![
            rep(1.0, 0.78, 0.0),
            rep(1.0, 0.77, 0.0),
            rep(0.99, 0.78, 0.0),
        ];
        let after = rep(0.80, 0.78, 0.0);
        assert!(is_regression(&history, &after, DEFAULT_K_SIGMA).is_some());
    }

    #[test]
    fn no_regression_within_noise() {
        let history = vec![rep(1.0, 0.78, 0.0), rep(0.99, 0.77, 0.0)];
        let after = rep(0.99, 0.78, 0.0);
        assert!(is_regression(&history, &after, DEFAULT_K_SIGMA).is_none());
    }

    #[test]
    fn fragment_rate_rise_is_flagged() {
        let history = vec![rep(1.0, 0.78, 0.0), rep(1.0, 0.78, 0.02)];
        let after = rep(1.0, 0.78, 0.40);
        assert!(is_regression(&history, &after, DEFAULT_K_SIGMA).is_some());
    }

    #[test]
    fn insufficient_history_does_not_gate() {
        let history = vec![rep(1.0, 0.78, 0.0)];
        let after = rep(0.10, 0.10, 0.9);
        assert!(is_regression(&history, &after, DEFAULT_K_SIGMA).is_none());
    }

    #[test]
    fn flat_history_floors_at_min_margin() {
        // Identical history => stdev 0; the margin floors to MIN_MARGIN (0.01),
        // so a sub-floor drop must not gate but a drop past it must.
        let history = vec![rep(1.0, 0.78, 0.0), rep(1.0, 0.78, 0.0)];
        let within = rep(0.991, 0.78, 0.0); // 0.009 drop, inside the floor
        assert!(is_regression(&history, &within, DEFAULT_K_SIGMA).is_none());
        let past = rep(0.989, 0.78, 0.0); // 0.011 drop, past the floor
        assert!(is_regression(&history, &past, DEFAULT_K_SIGMA).is_some());
    }
}
