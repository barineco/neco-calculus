# neco-rational

[日本語](README-ja.md)

`neco-rational` is a crate that provides exact rational functions formed from canonical polynomials. For both points and intervals, evaluation validates membership in the domain.

## Features

**Validated Rational Functions**

Construction keeps the canonical numerator and denominator polynomials and confirms that the denominator polynomial is nonzero.

**Exact Point Evaluation**

For values in the domain of the function, point evaluation gives an exact quotient.

**Validated Interval Evaluation**

The result of interval evaluation is an interval of dyadic rationals (rationals whose denominator is a power of two) that contains every quotient defined over the input interval.

**Typed Failures**

The variant of `RationalFailure` tells whether a failure arose in the domain, the polynomial, or the algebraic arithmetic, or came from exceeding a resource limit.

## Invariants and Validation Rules

| Target | Validation Rule | Result |
| --- | --- | --- |
| **Construction** | The denominator polynomial is a canonical nonzero polynomial | Returns `RationalFailure` when the denominator is the zero polynomial |
| **Point Evaluation** | The denominator value is nonzero | Returns a typed failure for an undefined point at a pole |
| **Interval Evaluation** | The denominator enclosure excludes zero | Returns a typed failure representing a domain violation for an interval containing a pole |
| **Resources** | Polynomial coefficient and precision limits hold | Keeps the failure that the dependency crate returns when a resource limit is exceeded |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-rational = "0.1"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Rational function and failure semantics are the same in both configurations.

```toml
[dependencies]
neco-rational = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical numerator and denominator polynomials.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact real algebraic evaluation values.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact dyadic interval arithmetic.

## License

MIT License. See the repository [LICENSE](../LICENSE) for details.
