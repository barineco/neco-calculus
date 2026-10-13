# neco-polynomial

[日本語](README-ja.md)

`neco-polynomial` provides canonical univariate polynomials over exact real algebraic coefficients, together with formal derivatives and validated interval evaluation.

## Features

**Canonical Polynomial Construction**

Constructing a polynomial removes trailing zero coefficients, which makes the coefficient representation unique. Every polynomial has at least one coefficient.

**Exact and Interval Evaluation**

Exact evaluation uses Horner's method. The result of interval evaluation is an interval containing every value the polynomial takes over the supplied interval of dyadic rationals (rationals whose denominator is a power of two).

**Typed Failures**

Construction, arithmetic, and interval failures, and failures from exceeding resource limits, are returned as distinct variants. Algebraic failures keep the payload returned by the crate that defines the operation.

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Construction** | Coefficients $\le 256$ | Returns `PolynomialFailure::Resource` for a larger coefficient sequence |
| **Differentiation** | Derivative order $\le 64$ | Returns `PolynomialFailure::Resource` for a larger order |
| **Interval Evaluation** | Precision $\le 256$ bits | Returns `PolynomialFailure::Resource` for a larger precision |
| **Canonicalization** | The coefficient sequence is nonempty | Returns `PolynomialFailure::EmptyCoefficients` for an empty sequence. Removes trailing zero coefficients and represents the zero polynomial with one coefficient |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-polynomial = "0.1"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Polynomial and failure semantics are the same in both configurations.

```toml
[dependencies]
neco-polynomial = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-algnum`](https://docs.rs/neco-algnum): Exact real algebraic coefficients and enclosures.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact integer and dyadic arithmetic.

## License

MIT License. See the repository [LICENSE](../LICENSE) for details.
