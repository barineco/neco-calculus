use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_jet::{DerivativeOrder, Jet, JetFailure, MAX_JET_ORDER};

fn algebraic(value: usize) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn assert_algebraic_integer(value: &RealAlgebraic, expected: usize) {
    assert_eq!(value, &algebraic(expected));
}

#[test]
fn rejects_empty_and_mismatched_derivatives() {
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), Vec::new()).unwrap_err(),
        JetFailure::EmptyDerivatives
    );
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), vec![algebraic(2)]).unwrap_err(),
        JetFailure::ValueDerivativeMismatch
    );
}

#[test]
fn stores_value_and_derivatives_by_order() {
    let jet = Jet::from_value_and_derivatives(
        algebraic(4),
        vec![algebraic(4), algebraic(4), algebraic(2)],
    )
    .unwrap();
    assert_eq!(jet.order(), DerivativeOrder::new(2));
    assert_algebraic_integer(jet.value(), 4);
    assert_algebraic_integer(jet.derivative(DerivativeOrder::new(0)).unwrap(), 4);
    assert_algebraic_integer(jet.derivative(DerivativeOrder::new(1)).unwrap(), 4);
    assert_algebraic_integer(jet.derivative(DerivativeOrder::new(2)).unwrap(), 2);
    assert_eq!(
        jet.derivative(DerivativeOrder::new(3)).unwrap_err(),
        JetFailure::OrderOutOfRange
    );
}

#[test]
fn shifts_to_first_derivative() {
    let jet = Jet::from_value_and_derivatives(
        algebraic(4),
        vec![algebraic(4), algebraic(4), algebraic(2)],
    )
    .unwrap();
    let shifted = jet.shift().unwrap();
    assert_eq!(shifted.order(), DerivativeOrder::new(1));
    assert_algebraic_integer(shifted.value(), 4);
    assert_algebraic_integer(shifted.derivative(DerivativeOrder::new(1)).unwrap(), 2);
}

#[test]
fn rejects_jet_order_above_the_public_limit() {
    let small = (0..=1).map(|_| algebraic(1)).collect();
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), small)
            .unwrap()
            .order(),
        DerivativeOrder::new(1)
    );
    let middle = (0..=8).map(|_| algebraic(1)).collect();
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), middle)
            .unwrap()
            .order(),
        DerivativeOrder::new(8)
    );
    let near_limit = (0..=MAX_JET_ORDER - 1).map(|_| algebraic(1)).collect();
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), near_limit)
            .unwrap()
            .order(),
        DerivativeOrder::new(MAX_JET_ORDER - 1)
    );
    let accepted = (0..=MAX_JET_ORDER).map(|_| algebraic(1)).collect();
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), accepted)
            .unwrap()
            .order(),
        DerivativeOrder::new(MAX_JET_ORDER)
    );
    let derivatives = (0..=MAX_JET_ORDER + 1).map(|_| algebraic(1)).collect();
    assert_eq!(
        Jet::from_value_and_derivatives(algebraic(1), derivatives),
        Err(JetFailure::OrderLimitExceeded {
            requested: MAX_JET_ORDER + 1,
            maximum: MAX_JET_ORDER,
        })
    );
}
