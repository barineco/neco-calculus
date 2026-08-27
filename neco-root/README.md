# neco-root

[日本語](README-ja.md)

`neco-root` provides verified root brackets, refinement evidence, multiplicity classification, and local membership analysis for exact polynomials.

## Features

**Verified Root Brackets**

A verified sign change or an exact even-multiplicity dyadic root establishes a root bracket. Public results expose observations while retaining private representations.

**Evidence-Preserving Refinement**

Bisection and Newton-bisection create convergence evidence. A derivative enclosure containing zero selects bisection and records the reason and count in the witness.

**Local Membership Analysis**

Candidate analysis establishes local roots and multiplicities. Interval evaluation establishes a positive or negative sign when its range excludes zero.

**Typed Failure Provenance**

Input, polynomial, interval, algebraic, and resource failures retain their distinct payloads through `RootFailure`.

## Specifications and Constraints

| Operation | Constraint | Result at the Limit |
| --- | --- | --- |
| **Candidate Analysis** | Candidate points $\le 4,096$ | Returns `RootFailure::ResourceLimit` for a larger request |
| **Multiplicity Analysis** | Derivative order $\le 64$ | Returns `RootFailure::ResourceLimit` for a larger order |
| **Refinement** | Iterations $\le 19$ | Returns `RootFailure::ResourceLimit` for a larger count |
| **Interval Evaluation** | Precision $\le 256$ bits | Returns `RootFailure::ResourceLimit` for a larger precision |
| **Classification** | Evidence establishes each public judgment | Produces an explicit undetermined result while evidence remains incomplete |

## Runtime Configuration

The default configuration integrates with the standard library.

```toml
[dependencies]
neco-root = "0.1"
```

Environments without the standard library use dynamic allocation alone. Result and failure semantics stay the same.

```toml
[dependencies]
neco-root = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Canonical polynomials, derivatives, and interval evaluation.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact algebraic values and enclosures.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact dyadic arithmetic.
* [`neco-formsum`](https://docs.rs/neco-formsum) and [`neco-monomial`](https://docs.rs/neco-monomial): Algebraic evidence forms.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
