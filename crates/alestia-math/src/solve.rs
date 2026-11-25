//! Bounded-iteration root-finding and fixed-point solvers matching and extending `celmath/solve.h`
//! (`FR-007`).

use core::fmt;

/// Result of a converged root or fixed-point solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolveResult {
    /// Approximated root or fixed point.
    pub root: f64,
    /// Estimated residual or interval half-width error.
    pub error: f64,
    /// Number of iterations executed.
    pub iterations: usize,
}

/// Structured errors returned by root solvers when inputs are invalid or convergence fails.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SolveError {
    /// An input bound, initial guess, tolerance, or function evaluation was `NaN`, `Inf`, or invalid (`tol <= 0`).
    NonFiniteInput,
    /// Bracket endpoints `[lower, upper]` are ordered incorrectly or do not bracket a root.
    InvalidBracket,
    /// Derivative became zero or non-finite during Newton or Halley step.
    ZeroDerivative,
    /// Iteration limit elapsed before reaching the requested tolerance.
    MaxIterationsExceeded {
        /// Latest root estimate when the limit was hit.
        last_estimate: f64,
        /// Latest error metric when the limit was hit.
        last_error: f64,
    },
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "non-finite input or invalid tolerance"),
            Self::InvalidBracket => write!(f, "invalid or non-bracketing interval"),
            Self::ZeroDerivative => write!(f, "zero derivative encountered in root solver"),
            Self::MaxIterationsExceeded {
                last_estimate,
                last_error,
            } => write!(
                f,
                "max iterations exceeded (estimate={last_estimate}, error={last_error})"
            ),
        }
    }
}

impl std::error::Error for SolveError {}

/// Solve `func(x) = 0` via bisection over `[lower, upper]` matching Celestia's `solve_bisection`
/// with strict non-finite and bracket validation.
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`] for non-finite bounds/tolerance/evaluations or `tolerance <= 0`,
/// [`SolveError::InvalidBracket`] when `lower > upper` or `func(lower)` and `func(upper)` share the same sign,
/// or [`SolveError::MaxIterationsExceeded`] if `max_iter` elapses before reaching `2 * tolerance`.
pub fn solve_bisection<F>(
    mut func: F,
    mut lower: f64,
    mut upper: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
{
    if !lower.is_finite() || !upper.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolveError::NonFiniteInput);
    }
    if lower > upper || max_iter == 0 {
        return Err(SolveError::InvalidBracket);
    }

    let mut f_lower = func(lower);
    let f_upper = func(upper);
    if !f_lower.is_finite() || !f_upper.is_finite() {
        return Err(SolveError::NonFiniteInput);
    }
    if f_lower == 0.0 {
        return Ok(SolveResult {
            root: lower,
            error: 0.0,
            iterations: 0,
        });
    }
    if f_upper == 0.0 {
        return Ok(SolveResult {
            root: upper,
            error: 0.0,
            iterations: 0,
        });
    }
    if f_lower.signum() == f_upper.signum() {
        return Err(SolveError::InvalidBracket);
    }

    let mut midpoint = f64::midpoint(lower, upper);
    for iter in 0..max_iter {
        midpoint = f64::midpoint(lower, upper);
        let half_width = 0.5 * (upper - lower);
        if upper - lower < 2.0 * tolerance {
            return Ok(SolveResult {
                root: midpoint,
                error: half_width,
                iterations: iter,
            });
        }
        let f_mid = func(midpoint);
        if !f_mid.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        if f_mid == 0.0 {
            return Ok(SolveResult {
                root: midpoint,
                error: 0.0,
                iterations: iter + 1,
            });
        }
        if f_mid.signum() == f_lower.signum() {
            lower = midpoint;
            f_lower = f_mid;
        } else {
            upper = midpoint;
        }
    }

    let final_error = 0.5 * (upper - lower);
    Err(SolveError::MaxIterationsExceeded {
        last_estimate: midpoint,
        last_error: final_error,
    })
}

/// Fixed-point iteration `x_{n+1} = func(x_n)` stopping when `|x_{n+1} - x_n| < tolerance`.
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`] on non-finite values or `tolerance <= 0`, or
/// [`SolveError::MaxIterationsExceeded`] if `max_iter` iterations elapse without converging.
pub fn solve_iteration<F>(
    mut func: F,
    initial: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
{
    if !initial.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolveError::NonFiniteInput);
    }
    let mut prev = initial;
    for iter in 1..=max_iter {
        let next = func(prev);
        if !next.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        let delta = next - prev;
        if delta.abs() < tolerance {
            return Ok(SolveResult {
                root: next,
                error: delta,
                iterations: iter,
            });
        }
        prev = next;
    }

    Err(SolveError::MaxIterationsExceeded {
        last_estimate: prev,
        last_error: f64::INFINITY,
    })
}

/// Fixed-count fixed-point iteration matching Celestia's `solve_iteration_fixed`.
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`] if `initial` or any iterate becomes non-finite.
pub fn solve_iteration_fixed<F>(
    mut func: F,
    initial: f64,
    iterations: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
{
    if !initial.is_finite() {
        return Err(SolveError::NonFiniteInput);
    }
    let mut prev = initial;
    let mut next = initial;
    for _ in 0..iterations {
        prev = next;
        next = func(prev);
        if !next.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
    }
    Ok(SolveResult {
        root: next,
        error: next - prev,
        iterations,
    })
}

/// Newton-Raphson solver for `func(x) = 0` given analytic derivative `deriv(x)`.
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`], [`SolveError::ZeroDerivative`], or
/// [`SolveError::MaxIterationsExceeded`].
pub fn solve_newton<F, D>(
    mut func: F,
    mut deriv: D,
    initial: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
    D: FnMut(f64) -> f64,
{
    if !initial.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolveError::NonFiniteInput);
    }

    let mut estimate = initial;
    let mut last_error = f64::INFINITY;
    for iter in 1..=max_iter {
        let fx = func(estimate);
        let dfx = deriv(estimate);
        if !fx.is_finite() || !dfx.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        if dfx == 0.0 {
            return Err(SolveError::ZeroDerivative);
        }
        let step = fx / dfx;
        let next = estimate - step;
        if !next.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        last_error = step.abs();
        estimate = next;
        if last_error < tolerance {
            return Ok(SolveResult {
                root: estimate,
                error: last_error,
                iterations: iter,
            });
        }
    }

    Err(SolveError::MaxIterationsExceeded {
        last_estimate: estimate,
        last_error,
    })
}

/// Halley's third-order root finder for `func(x) = 0` given `deriv` (`f'`) and `second_deriv` (`f''`).
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`], [`SolveError::ZeroDerivative`], or
/// [`SolveError::MaxIterationsExceeded`].
pub fn solve_halley<F, D, D2>(
    mut func: F,
    mut deriv: D,
    mut second_deriv: D2,
    initial: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
    D: FnMut(f64) -> f64,
    D2: FnMut(f64) -> f64,
{
    if !initial.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolveError::NonFiniteInput);
    }

    let mut estimate = initial;
    let mut last_error = f64::INFINITY;
    for iter in 1..=max_iter {
        let val = func(estimate);
        let first_deriv = deriv(estimate);
        let second_val = second_deriv(estimate);
        if !val.is_finite() || !first_deriv.is_finite() || !second_val.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        let denom = 2.0 * first_deriv * first_deriv - val * second_val;
        if denom == 0.0 {
            return Err(SolveError::ZeroDerivative);
        }
        let step = (2.0 * val * first_deriv) / denom;
        let next = estimate - step;
        if !next.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
        last_error = step.abs();
        estimate = next;
        if last_error < tolerance {
            return Ok(SolveResult {
                root: estimate,
                error: last_error,
                iterations: iter,
            });
        }
    }

    Err(SolveError::MaxIterationsExceeded {
        last_estimate: estimate,
        last_error,
    })
}

struct BrentState {
    left_x: f64,
    right_x: f64,
    contra_x: f64,
    left_y: f64,
    right_y: f64,
    contra_y: f64,
    step: f64,
    prev_step: f64,
}

impl BrentState {
    fn compute_step(&mut self, half_interval: f64, tol_act: f64) {
        if self.prev_step.abs() >= tol_act && self.left_y.abs() > self.right_y.abs() {
            let ratio = self.right_y / self.left_y;
            let (mut num, mut den) = if (self.left_x - self.contra_x).abs() <= f64::EPSILON {
                (2.0 * half_interval * ratio, 1.0 - ratio)
            } else {
                let ratio_c = self.left_y / self.contra_y;
                let ratio_r = self.right_y / self.contra_y;
                let p_iqi = ratio
                    * (2.0 * half_interval * ratio_c * (ratio_c - ratio_r)
                        - (self.right_x - self.left_x) * (ratio_r - 1.0));
                let q_iqi = (ratio_c - 1.0) * (ratio_r - 1.0) * (ratio - 1.0);
                (p_iqi, q_iqi)
            };

            if num > 0.0 {
                den = -den;
            } else {
                num = -num;
            }

            let bound1 = 3.0 * half_interval * den - (tol_act * den).abs();
            let bound2 = (self.prev_step * den).abs();
            if 2.0 * num < bound1.min(bound2) {
                self.prev_step = self.step;
                self.step = num / den;
                return;
            }
        }
        self.step = half_interval;
        self.prev_step = half_interval;
    }
}

/// Brent's root-finding method (inverse quadratic interpolation, secant, and bisection hybrid).
///
/// # Errors
/// Returns [`SolveError::NonFiniteInput`], [`SolveError::InvalidBracket`], or
/// [`SolveError::MaxIterationsExceeded`].
pub fn solve_brent<F>(
    mut func: F,
    lower: f64,
    upper: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolveResult, SolveError>
where
    F: FnMut(f64) -> f64,
{
    if !lower.is_finite() || !upper.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolveError::NonFiniteInput);
    }
    if lower > upper || max_iter == 0 {
        return Err(SolveError::InvalidBracket);
    }

    let left_y = func(lower);
    let right_y = func(upper);
    if !left_y.is_finite() || !right_y.is_finite() {
        return Err(SolveError::NonFiniteInput);
    }
    if left_y == 0.0 {
        return Ok(SolveResult {
            root: lower,
            error: 0.0,
            iterations: 0,
        });
    }
    if right_y == 0.0 {
        return Ok(SolveResult {
            root: upper,
            error: 0.0,
            iterations: 0,
        });
    }
    if left_y.signum() == right_y.signum() {
        return Err(SolveError::InvalidBracket);
    }

    let mut state = BrentState {
        left_x: lower,
        right_x: upper,
        contra_x: lower,
        left_y,
        right_y,
        contra_y: left_y,
        step: upper - lower,
        prev_step: upper - lower,
    };

    for iter in 1..=max_iter {
        if state.right_y.signum() == state.contra_y.signum() {
            state.contra_x = state.left_x;
            state.contra_y = state.left_y;
            state.step = state.right_x - state.left_x;
            state.prev_step = state.step;
        }
        if state.contra_y.abs() < state.right_y.abs() {
            state.left_x = state.right_x;
            state.right_x = state.contra_x;
            state.contra_x = state.left_x;
            state.left_y = state.right_y;
            state.right_y = state.contra_y;
            state.contra_y = state.left_y;
        }

        let tol_act = 2.0 * f64::EPSILON * state.right_x.abs() + 0.5 * tolerance;
        let half_interval = 0.5 * (state.contra_x - state.right_x);

        if half_interval.abs() <= tol_act || state.right_y == 0.0 {
            return Ok(SolveResult {
                root: state.right_x,
                error: half_interval.abs(),
                iterations: iter,
            });
        }

        state.compute_step(half_interval, tol_act);
        state.left_x = state.right_x;
        state.left_y = state.right_y;
        if state.step.abs() > tol_act {
            state.right_x += state.step;
        } else if half_interval > 0.0 {
            state.right_x += tol_act;
        } else {
            state.right_x -= tol_act;
        }

        state.right_y = func(state.right_x);
        if !state.right_y.is_finite() {
            return Err(SolveError::NonFiniteInput);
        }
    }

    Err(SolveError::MaxIterationsExceeded {
        last_estimate: state.right_x,
        last_error: 0.5 * (state.contra_x - state.right_x).abs(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SolveError, solve_bisection, solve_brent, solve_halley, solve_iteration,
        solve_iteration_fixed, solve_newton,
    };

    #[test]
    fn bisection_and_brent_converge_to_1e13() -> Result<(), String> {
        let cubic = |x: f64| x * x * x - x - 2.0;
        let bis = solve_bisection(cubic, 1.0, 2.0, 1e-14, 100)
            .map_err(|err| format!("Bisection failed: {err}"))?;
        let brent = solve_brent(cubic, 1.0, 2.0, 1e-14, 100)
            .map_err(|err| format!("Brent failed: {err}"))?;
        if (bis.root - brent.root).abs() > 1e-13 || cubic(brent.root).abs() > 1e-13 {
            return Err(format!("Inaccurate root: bis={bis:?}, brent={brent:?}"));
        }
        Ok(())
    }

    #[test]
    fn newton_and_halley_solve_kepler_equation() -> Result<(), String> {
        let mean_anomaly = 0.25_f64;
        let ecc = 0.967_14_f64; // Halley's Comet
        let f = |e: f64| e - ecc * e.sin() - mean_anomaly;
        let df = |e: f64| 1.0 - ecc * e.cos();
        let d2f = |e: f64| ecc * e.sin();

        let newton =
            solve_newton(f, df, 1.0, 1e-14, 30).map_err(|err| format!("Newton failed: {err}"))?;
        let halley = solve_halley(f, df, d2f, 1.0, 1e-14, 20)
            .map_err(|err| format!("Halley failed: {err}"))?;
        if f(newton.root).abs() > 1e-13 || f(halley.root).abs() > 1e-13 {
            return Err("Kepler solver exceeded tolerance".to_owned());
        }
        if (newton.root - halley.root).abs() > 1e-13 {
            return Err("Newton and Halley diverged".to_owned());
        }
        Ok(())
    }

    #[test]
    fn fixed_point_solvers_converge_on_cosine() -> Result<(), String> {
        let iter = solve_iteration(f64::cos, 0.5, 1e-12, 200)
            .map_err(|err| format!("Iteration failed: {err}"))?;
        if (iter.root - iter.root.cos()).abs() > 1e-11 {
            return Err("Cosine fixed point inaccurate".to_owned());
        }
        let fixed = solve_iteration_fixed(f64::cos, 0.5, 60)
            .map_err(|err| format!("Fixed iteration failed: {err}"))?;
        if (fixed.root - iter.root).abs() > 1e-9 {
            return Err("Fixed iteration mismatch".to_owned());
        }
        Ok(())
    }

    #[test]
    fn solvers_guard_nan_invalid_brackets_zero_deriv_and_max_iter() {
        assert_eq!(
            solve_brent(|x| x, f64::NAN, 1.0, 1e-6, 10),
            Err(SolveError::NonFiniteInput)
        );
        assert_eq!(
            solve_bisection(|x| x * x + 1.0, 0.0, 2.0, 1e-6, 20),
            Err(SolveError::InvalidBracket)
        );
        assert_eq!(
            solve_newton(|x| x * x, |_| 0.0, 0.0, 1e-6, 10),
            Err(SolveError::ZeroDerivative)
        );
        assert!(matches!(
            solve_newton(|x| x * x - 2.0, |x| 2.0 * x, 10.0, 1e-15, 1),
            Err(SolveError::MaxIterationsExceeded { .. })
        ));
        assert_eq!(
            solve_halley(|x| x, |_| 0.0, |_| 0.0, 1.0, 1e-6, 5),
            Err(SolveError::ZeroDerivative)
        );
        assert_eq!(
            solve_iteration(|_| f64::NAN, 0.0, 1e-6, 5),
            Err(SolveError::NonFiniteInput)
        );
    }
}
