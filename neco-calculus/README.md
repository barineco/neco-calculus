# neco-calculus

[日本語](README-ja.md)

`neco-calculus` is the public operation facade for exact polynomial and rational calculus. It combines exact evaluation, validated enclosures, finite derivative sequences, root refinement, and local membership analysis while preserving typed result provenance.

## Features

**Exact Shape Polynomial Evaluation**

Surface and implicit polynomial constructors produce canonical term order. Evaluation returns exact values or validated enclosures through opaque result types.

**Rational, Jet, and Root Operations**

Facade operations expose rational evaluation, derivative sequences, root refinement, and membership analysis while retaining dependency failures and their payloads.

**Validated Transcendental Enclosures**

The connector validates expression identity, precision, and enclosure width before an evaluation operation creates a result and its provenance.

**Observable Result Provenance**

Enclosure results retain the evaluation method and source expression. Accessors return them together with the computed enclosure.

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Surface Polynomial Construction** | Terms $\le 4,096$ | Returns `CalculusFailure` for a larger input |
| **Implicit Polynomial Construction** | Terms $\le 4,096$ | Returns `CalculusFailure` for a larger input |
| **Derivative and Root Operations** | Dependency order, candidate, iteration, and precision limits hold | Preserves the typed dependency resource failure |
| **Transcendental Validation** | Expression identity, requested precision, and enclosure width agree | Returns `TranscendentalConnectorFailure` for an invalid candidate |
| **Result Construction** | A calculus operation supplies the result provenance | Produces an opaque result with observable provenance |

## Runtime Configuration

The default configuration integrates with the standard library and enables standard-library support in every dependency.

```toml
[dependencies]
neco-calculus = "0.1"
```

Environments without the standard library use dynamic allocation alone. Public values, results, and failure meanings stay the same.

```toml
[dependencies]
neco-calculus = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical polynomials and validated interval evaluation.
* [`neco-rational`](https://docs.rs/neco-rational): Exact rational functions and domain validation.
* [`neco-jet`](https://docs.rs/neco-jet): Finite derivative sequences.
* [`neco-root`](https://docs.rs/neco-root): Root and local membership evidence.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact values.
* [`neco-bigint`](https://docs.rs/neco-bigint): Dyadic arithmetic.
* [`neco-expr`](https://docs.rs/neco-expr): Stable expression identities.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
