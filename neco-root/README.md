# neco-root

[日本語](README-ja.md)

`neco-root` is a crate that provides verified root brackets, root bracket narrowing, multiplicity classification, interval sign classification, and candidate root classification for exact polynomials.

## Features

**Verified Root Brackets**

A root bracket (an interval verified to contain a root) is established on the basis of either a verified sign change or an even-multiplicity root that is exactly representable as a dyadic rational (a rational whose denominator is a power of two). Public result types hide their internal representation, and their contents are returned through accessors.

**Root Bracket Narrowing and Narrowing Witnesses**

Root bracket narrowing shrinks a root bracket by bisection or Newton-bisection and creates a narrowing witness (`ConvergenceWitness`). The narrowing witness holds the method, the convergence status, the iteration counts, and the initial and final intervals. When a derivative enclosure contains zero during Newton-bisection, it switches to bisection and records the reason and count in the narrowing witness.

**Interval Sign Classification and Candidate Root Classification**

Both operations take a request (`MembershipJudgmentRequest`) that bundles the polynomial, the interval, the candidate points, the derivative order limit, and the interval evaluation precision. Candidate points are dyadic points that the caller selects from the interval.

`classify_region_sign` classifies the sign of the polynomial on the interval and returns the classification result (`RegionSignJudgment`).

Its `sign` returns the classified sign as a `RegionSign` value. For a constant polynomial, the classified sign is the sign of its coefficient.

For a polynomial of degree one or more, the classification first finds which candidate points are roots of the polynomial. If an interior candidate point is an odd-multiplicity root, the sign is classified as changing within the interval. If the roots are only endpoint roots or even-multiplicity roots, the polynomial is classified as taking zero within the interval (`RegionSign::ContainsZero`).

When no candidate point is a root, interval evaluation computes the range of the polynomial on the interval. If that range lies entirely on the positive side or entirely on the negative side, the sign is established as positive or negative.

When the range contains zero, the signs of the polynomial values at the two endpoints are compared. If one is positive and the other negative, the sign is classified as changing within the interval. In the remaining cases, including a polynomial value of zero at an endpoint, the sign stays undetermined and the result is `RegionSign::Undetermined`.

`locate_membership_transitions` establishes which candidate points are roots of the polynomial, together with their multiplicities.

It classifies an odd-multiplicity root as a transition (`Transition`), where the sign changes, and an even-multiplicity root as a contact point (`SingularContact`), where the polynomial touches zero and keeps its sign.

**Typed Failures**

`RootFailure` keeps input, polynomial, interval, and algebraic failures, and failures from exceeding resource limits, each with its payload.

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Candidate Analysis** | Candidate points $\le 4,096$ | Returns `RootFailure::ResourceLimit` for a larger request |
| **Multiplicity Analysis** | Derivative order $\le 64$ | Returns `RootFailure::ResourceLimit` for a larger order |
| **Root Bracket Narrowing** | Iterations $\le 19$ | Returns `RootFailure::ResourceLimit` for a larger count |
| **Interval Evaluation** | Precision $\le 256$ bits | Returns `RootFailure::ResourceLimit` for a larger precision |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-root = "0.1"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Result and failure semantics are the same in both configurations.

```toml
[dependencies]
neco-root = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical polynomials, derivatives, and interval evaluation.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact algebraic values and enclosures.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact dyadic arithmetic.
* [`neco-formsum`](https://docs.rs/neco-formsum) and [`neco-monomial`](https://docs.rs/neco-monomial): Formal sums and monomials used to convert dyadic points to real algebraic numbers.

## License

MIT License. See the repository [LICENSE](../LICENSE) for details.
