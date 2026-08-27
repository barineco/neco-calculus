# neco-polynomial

[日本語](README-ja.md)

`neco-polynomial` provides canonical univariate polynomials over exact real algebraic coefficients, together with formal derivatives and validated interval evaluation.

## Features

**Canonical Polynomial Construction**

Construction produces a unique coefficient representation by removing trailing zero coefficients. Every polynomial retains at least one coefficient.

**Exact and Interval Evaluation**

Exact evaluation uses Horner form. Interval evaluation returns an enclosure containing every polynomial value over the supplied dyadic interval.

**Typed Failure Provenance**

Construction, arithmetic, interval, and resource failures remain distinct. Algebraic failures preserve the payload supplied by their defining crate.

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Construction** | Coefficients $\le 256$ | Returns `PolynomialFailure::Resource` for a larger sequence |
| **Differentiation** | Derivative order $\le 64$ | Returns `PolynomialFailure::Resource` for a larger order |
| **Interval Evaluation** | Precision $\le 256$ bits | Returns `PolynomialFailure::Resource` for a larger precision |
| **Canonicalization** | A coefficient sequence contains a canonical nonempty value | Removes trailing zeros and represents zero with one coefficient |

## Runtime Configuration

The default configuration integrates with the standard library.

```toml
[dependencies]
neco-polynomial = "0.1"
```

Environments without the standard library use dynamic allocation alone. Polynomial and failure semantics stay the same.

```toml
[dependencies]
neco-polynomial = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-algnum`](https://docs.rs/neco-algnum): Exact real algebraic coefficients and enclosures.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact integer and dyadic arithmetic.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
