# neco-rational

[日本語](README-ja.md)

`neco-rational` provides exact rational functions formed from canonical polynomials, with validated point and interval domains.

## Features

**Validated Rational Functions**

Construction stores canonical numerator and denominator polynomials and establishes a nonzero denominator polynomial.

**Exact Point Evaluation**

Point evaluation returns an exact quotient for values in the function domain.

**Validated Interval Evaluation**

Interval evaluation returns a dyadic enclosure containing every defined quotient over the input interval.

**Typed Failure Provenance**

Domain, polynomial, algebraic, and resource failures remain distinguishable through `RationalFailure`.

## Invariants and Validation Rules

| Target | Validation Rule | Result |
| --- | --- | --- |
| **Construction** | Denominator polynomial has a nonzero canonical value | Returns `RationalFailure` for the zero polynomial |
| **Point Evaluation** | Denominator value has a nonzero value | Returns a typed domain failure at a pole |
| **Interval Evaluation** | Denominator enclosure excludes zero | Returns a typed domain failure for an interval containing a pole |
| **Resources** | Polynomial coefficient and precision limits hold | Preserves the dependency resource failure |

## Runtime Configuration

The default configuration integrates with the standard library.

```toml
[dependencies]
neco-rational = "0.1"
```

Environments without the standard library use dynamic allocation alone. Function and failure semantics stay the same.

```toml
[dependencies]
neco-rational = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical numerator and denominator polynomials.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact real algebraic evaluation values.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact dyadic interval arithmetic.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
