# neco-jet

[日本語](README-ja.md)

`neco-jet` provides finite sequences of exact derivatives at one point, with validated order and value consistency.

## Features

**Validated Derivative Sequences**

Construction establishes a nonempty derivative sequence whose order-zero entry equals the supplied value.

**Order-Preserving Observation**

Accessors retain derivative order. Shifting returns the sequence beginning at the first derivative.

**Typed Failure Provenance**

Sequence, order, polynomial, and algebraic failures remain distinguishable through `JetFailure`.

## Invariants and Validation Rules

| Target | Validation Rule | Result |
| --- | --- | --- |
| **Sequence** | Contains at least the order-zero value | Returns `JetFailure` for an empty sequence |
| **Order-Zero Value** | Equals the separately supplied value | Returns `JetFailure` for a mismatch |
| **Derivative Order** | Order $\le 64$ | Returns `JetFailure` before storing a larger sequence |
| **Shift** | Preserves the original derivative ordering | Produces the tail beginning at the first derivative |

## Runtime Configuration

The default configuration integrates with the standard library.

```toml
[dependencies]
neco-jet = "0.1"
```

Environments without the standard library use dynamic allocation alone. Jet and failure semantics stay the same.

```toml
[dependencies]
neco-jet = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): Polynomial derivatives and associated failures.
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact derivative values.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
