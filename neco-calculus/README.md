# neco-calculus

[日本語](README-ja.md)

`neco-calculus` is the crate that gathers the public operations for exact polynomial and rational calculus. It combines exact evaluation, finite derivative sequences, validated enclosures (intervals verified to contain the exact value), root bracket narrowing, interval sign classification, and candidate root classification.

Root bracket narrowing shrinks an interval that contains a root by bisection or Newton-bisection. The process is kept in a narrowing witness (`ConvergenceWitness`) that holds the method, the convergence status, the iteration counts, and the initial and final intervals.

Interval sign classification determines whether the sign of a polynomial is constant on an interval or changes within it. Candidate root classification takes the candidate points supplied by the caller, keeps those that are roots of the polynomial, and separates odd-multiplicity roots, where the sign changes, from even-multiplicity roots, where the sign stays the same.

## Features

**Exact Evaluation of Surface and Implicit Polynomials**

A surface polynomial (`SurfacePolynomial`) is a polynomial in the variables u and v with rational coefficients. A rational parametric surface represents its x, y, and z coordinates and their common denominator by surface polynomials. An implicit polynomial (`ImplicitPolynomial3`) is a polynomial in the variables x, y, and z with real algebraic coefficients.

Both constructions arrange the terms in ascending lexicographic order of their exponent tuples. Surface polynomial construction adds terms with the same exponent tuple and removes terms whose coefficient is zero.

Implicit polynomial construction returns a failure for an input with a zero coefficient or a repeated exponent tuple. The evaluation functions return exact values or validated enclosures wrapped in result types that hide their internal representation.

**Rational, Jet, and Root Operations**

The crate also exposes rational evaluation and derivative sequences (`Jet`). Root operations have public functions for root bracket narrowing, interval sign classification, and candidate root classification.

* `refine_root_bisection`: Narrows a root bracket by bisection.
* `refine_root_newton_bisection`: Narrows a root bracket by Newton-bisection.
* `expose_region_sign`: Returns the interval sign classification result (`RegionSignJudgment`).
* `expose_membership_transitions`: Returns the candidate root classification.

Failures returned by dependency crates are kept as they are, including the values attached to them.

**Transcendental Evaluation**

Sine, cosine, exponential, and real powers are evaluated over interval arguments at a requested absolute precision. When the input interval contains a point where sine or cosine takes an extremum, the enclosure also contains that extremum. Whether such a point lies in the input interval is decided with an outward-rounded enclosure of pi.

For a requested precision of p bits, the target width is 2^-p. A result is an exact value, an enclosure no wider than the target width, or an undetermined result.

Evaluation estimates the width of the result enclosure. For sine and cosine, the width estimate is the width of the computed enclosure.

For the exponential, the width estimate is the width of the argument interval multiplied by an upper bound of the derivative magnitude. For real powers, the width estimate is the sum, over the base and the exponent, of the interval width multiplied by an upper bound of the partial derivative magnitude.

An undetermined result appears when the width estimate exceeds the target width, and it keeps the interval established so far. The actual width of that interval can still be within the target width.

* `evaluate_transcendental_function`: Evaluates the function selected by `TranscendentalFunction`.
* `evaluate_transcendental_power`: Evaluates a real power whose base is a strictly positive interval.
* `TranscendentalEvaluation::undetermined_achieved`: Returns the interval kept by an undetermined result.
* `TranscendentalFunction`: Names the functions that can be evaluated.
  * `Sine`
  * `Cosine`
  * `Exponential`

**Certified Double-Precision Projection**

A result can be projected to the nearest double-precision floating-point value (`f64`). The projected value comes with the enclosure before rounding and the actual absolute error of that rounding. The size of the error depends on the width of the enclosure (at most the target width) and the spacing of adjacent double-precision values.

* `TranscendentalEvaluation::certify_f64`: Returns a `CertifiedF64` for an exact value or an enclosure.

**Validated Transcendental Enclosures**

`construct_transcendental_evaluation` builds a transcendental evaluation result from an evaluation candidate supplied by the caller. An evaluation candidate (`TranscendentalEvaluationCandidate`) pairs a value with its provenance.

Before construction, the function validates that the candidate provenance matches the provenance the caller requires, that the requested precision is from 1 to 256 bits, and that the enclosure is no wider than the target width.

**Certified Quadrature**

Quadrature integrates over a closed interval whose bounds are dyadic rationals (rationals whose denominator is a power of two) and returns an enclosure of the integral. For a requested precision of p bits, the width of the enclosure is at most the target width 2^-p.

Quadrature divides the integration interval into subintervals and expands the integrand in a Taylor series at the midpoint of each subinterval. The quadrature order (the value of `QuadratureOrder`) is the number of even-order terms of this expansion kept before the remainder term. The remainder-term enclosure is computed from the range that the derivative whose order is twice the quadrature order takes over the subinterval.

The integrand implements `IntervalDifferentiable` and returns interval enclosures of its values and of its even-order derivatives. A quadrature result contains the number of subintervals and the sum of the widths of the remainder-term enclosures.

* `evaluate_quadrature`: Encloses the integral of an integrand over a closed interval.
* `IntervalDifferentiable`: The trait through which an integrand returns enclosures of its values and derivatives.
* `QuadratureOrder`: Sets the quadrature order.
* `QuadratureRefinement`: Sets the quadrature order and the maximum number of subintervals.
* `QuadratureEvaluation::certify_f64`: Returns the certified double-precision value of the enclosure.

**Observable Result Provenance**

Results of transcendental evaluation, checked surface evaluation (`evaluate_surface_checked`), and quadrature keep their provenance. The provenance consists of the owner identifier, the evaluation requirement, and the implementation identity.

The evaluation requirement contains the consumer identifier (the place that receives the result), the list of expression identifiers, the decision request, and the requested precision. The decision request specifies which exact decision (zero, equality, sign, or degeneracy) the consumer requests on which of the listed expressions (two expressions for equality). An expression identifier (`ExprId`) names a node of an expression graph in neco-expr.

The implementation identity pairs a version with a public API identifier. Accessors return each part of the provenance as well as the result value.

* `owner`: The owner identifier (`OwnerId`).
* `requirement`: The evaluation requirement (`ExactExpressionRequirement`).
* `source`: The implementation identity (`NecoImplementationSource`).

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Surface Polynomial Construction** | Terms $\le 4,096$ and each exponent $\le 256$ | Returns `CalculusFailure` for a larger input |
| **Implicit Polynomial Construction** | Terms $\le 4,096$ | Returns `CalculusFailure` for a larger input |
| **Derivative and Root Operations** | The order, candidate, iteration, and precision limits set by the dependency crates hold | Keeps the typed failure that a dependency crate returns when a resource limit is exceeded |
| **Transcendental Evaluation** | Precision from $1$ to $256$ bits and a base above zero | Returns `EvaluationFailure` for any other precision or base |
| **Series Term Limit** | The number of series terms, determined by the argument magnitude and the working precision (96 bits more than the requested precision), is at most $256$ | Returns `EvaluationFailure` that wraps the typed failure of the dependency crate |
| **Undetermined Result** | The width estimate exceeds the target width | Returns the established interval as a success value; its width can still meet the target |
| **Double-Precision Projection** | The result holds an exact value or an enclosure | Returns `TranscendentalCertificationFailure` for an undetermined result |
| **Transcendental Validation** | The candidate provenance matches the request, the requested precision is from $1$ to $256$ bits, and the enclosure width is at most the target width | Returns `EvaluationFailure::Lower` for a candidate whose provenance differs from the request. Returns `EvaluationFailure::PrecisionShortage` when the requested precision is out of range or the enclosure is wider than the target width |
| **Quadrature** | The lower bound is below the upper bound and the requested bits are from $1$ to $256$ | Returns a typed failure; when the width remains larger than requested, the failure keeps the enclosure reached |
| **Subinterval Division** | Twice the order $\le 64$ and subintervals $\le 65,536$ | Returns `CalculusFailure` for a larger order or number of subintervals |
| **Result Construction** | The struct result types (polynomial evaluation, rational evaluation, the evaluation of a polynomial with rational coefficients, interval evaluation, transcendental evaluation results, coordinate enclosures, and quadrature results) have private fields | Only public functions of this crate create these results. Transcendental evaluation results, the coordinate enclosures returned by checked surface evaluation, and quadrature results carry provenance, and that provenance is observable. The enums returned by surface and implicit evaluation hold an exact value in public fields when the result is exact. For checked surface evaluation, the enum holds the coordinate enclosure |

## Runtime Configuration

The default configuration uses the standard library and also enables standard-library support in every dependency crate.

```toml
[dependencies]
neco-calculus = "0.2"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Public values, results, and failure meanings are the same in both configurations.

```toml
[dependencies]
neco-calculus = { version = "0.2", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical polynomials and validated interval evaluation.
* [`neco-rational`](https://docs.rs/neco-rational): Exact rational functions and domain validation.
* [`neco-jet`](https://docs.rs/neco-jet): Finite derivative sequences.
* [`neco-root`](https://docs.rs/neco-root): Root brackets, narrowing witnesses, interval sign classification, and candidate root classification.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact values.
* [`neco-bigint`](https://docs.rs/neco-bigint): Dyadic arithmetic.
* [`neco-expr`](https://docs.rs/neco-expr): Identifiers that make up provenance, and certified `f64` values.

## License

MIT License. See the repository [LICENSE](../LICENSE) for details.
