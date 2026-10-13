# neco-jet

[日本語](README-ja.md)

`neco-jet` is a crate that provides finite sequences of exact derivative values at one point. Construction validates that the derivative order is within the limit and that the function value equals the order-zero entry.

## Features

**Validated Derivative Sequences**

`Jet::from_value_and_derivatives` builds a derivative sequence from the function value at the point (`value`) and a nonempty sequence of derivative values. Construction confirms that the order-zero entry of the sequence equals the function value.

**Order-Preserving Observation**

Shifting returns the sequence beginning at the first derivative. Each entry read through an accessor stays associated with its derivative order.

**Typed Failures**

The variant of `JetFailure` identifies the cause of a failure: an empty sequence, a mismatch between the function value and the order-zero entry, a reference to an order beyond the sequence, an order above the limit, or a failure in algebraic arithmetic.

## Invariants and Validation Rules

| Target | Validation Rule | Result |
| --- | --- | --- |
| **Sequence** | Contains at least the order-zero value | Returns `JetFailure` for an empty sequence |
| **Order-Zero Value** | Equals the function value passed at construction | Returns `JetFailure` when the values differ |
| **Derivative Order** | Order $\le 64$ | Returns `JetFailure` for a larger sequence before storing it |
| **Shift** | Keeps the original derivative order | Produces the remaining sequence beginning at the first derivative |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-jet = "0.1"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Derivative sequence and failure semantics are the same in both configurations.

```toml
[dependencies]
neco-jet = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-polynomial`](https://docs.rs/neco-polynomial): The derivative order type (`DerivativeOrder`).
* [`neco-algnum`](https://docs.rs/neco-algnum): Exact derivative values.

## License

MIT License. See the repository [LICENSE](../LICENSE) for details.
