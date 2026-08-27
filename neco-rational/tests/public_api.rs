use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_polynomial::{Enclosure, Polynomial};
use neco_rational::{RationalFailure, RationalFunction};

fn algebraic(value: usize) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i64) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), 0)
}

fn assert_algebraic_integer(value: &RealAlgebraic, expected: usize) {
    assert_eq!(value, &algebraic(expected));
}

fn x_polynomial() -> Polynomial {
    Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap()
}

#[test]
fn rejects_zero_denominator_polynomial() {
    let numerator = Polynomial::constant(algebraic(1)).unwrap();
    let denominator = Polynomial::zero().unwrap();
    assert_eq!(
        RationalFunction::new(numerator, denominator).unwrap_err(),
        RationalFailure::ZeroDenominatorPolynomial
    );
}

#[test]
fn evaluates_points_and_rejects_zero_domain() {
    let function =
        RationalFunction::new(Polynomial::constant(algebraic(4)).unwrap(), x_polynomial()).unwrap();
    assert_eq!(
        function.evaluate(&algebraic(0)).unwrap_err(),
        RationalFailure::UndefinedDomain
    );
    assert_algebraic_integer(&function.evaluate(&algebraic(2)).unwrap(), 2);
}

#[test]
fn evaluates_intervals_with_domain_checks() {
    let function =
        RationalFunction::new(Polynomial::constant(algebraic(4)).unwrap(), x_polynomial()).unwrap();
    let containing_zero = Enclosure::new(dyadic(-1), dyadic(1)).unwrap();
    assert_eq!(
        function.evaluate_on(&containing_zero, 8).unwrap_err(),
        RationalFailure::DomainContainsZero
    );

    let positive = Enclosure::new(dyadic(1), dyadic(2)).unwrap();
    let image = function.evaluate_on(&positive, 8).unwrap();
    assert!(image.contains(&dyadic(2)));
    assert!(image.contains(&dyadic(4)));
}
