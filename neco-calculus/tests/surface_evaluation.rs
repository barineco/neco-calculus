use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{
    evaluate_surface, evaluate_surface_checked, CalculusFailure, DomainWitness, Enclosure,
    EvaluationFailure, EvaluationOperation, OwnerId, RationalFailure, RationalSurfacePolynomials,
    SurfaceEvaluationInput, SurfaceEvaluationInput2, SurfaceEvaluationProducer, SurfacePolynomial,
    SurfaceTerm, MAX_PRECISION_BITS, MAX_SURFACE_EXPONENT, MAX_SURFACE_TERMS,
};
use neco_expr::{
    AbsoluteBits, ConsumerId, ExactDecisionRequest, ExactExpressionRequirement, ExprId,
    NecoImplementationSource,
};

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i32, exponent: u32) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), exponent)
}

fn surface_term(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfaceTerm {
    SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }
}

fn polynomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![surface_term(coefficient, exponent_u, exponent_v)])
        .unwrap()
}

fn constant(coefficient: i32) -> SurfacePolynomial {
    polynomial(coefficient, 0, 0)
}

fn interval(lower: i32, upper: i32) -> Enclosure {
    Enclosure::new(dyadic(lower, 0), dyadic(upper, 0)).unwrap()
}

fn exact_input(
    x: SurfacePolynomial,
    y: SurfacePolynomial,
    z: SurfacePolynomial,
    denominator: SurfacePolynomial,
) -> SurfaceEvaluationInput {
    SurfaceEvaluationInput {
        x,
        y,
        z,
        denominator,
        u: algebraic(2),
        v: algebraic(3),
    }
}

fn requirement(precision_bits: u32) -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(11),
        &[ExprId::new(500)],
        ExactDecisionRequest::Zero(ExprId::new(500)),
        AbsoluteBits::new(precision_bits),
    )
    .unwrap()
}

fn checked_input(
    x: SurfacePolynomial,
    y: SurfacePolynomial,
    z: SurfacePolynomial,
    denominator: SurfacePolynomial,
    u: Enclosure,
    v: Enclosure,
    precision_bits: u32,
) -> Result<SurfaceEvaluationInput2, CalculusFailure> {
    SurfaceEvaluationInput2::new(
        RationalSurfacePolynomials {
            x,
            y,
            z,
            denominator,
        },
        u,
        v,
        OwnerId::new(1),
        requirement(precision_bits),
        NecoImplementationSource::current(),
    )
}

fn certified_coordinates(
    producer: SurfaceEvaluationProducer,
) -> neco_calculus::CoordinateEnclosure3 {
    match producer {
        SurfaceEvaluationProducer::Certified { coordinates } => *coordinates,
        SurfaceEvaluationProducer::Exact { .. } => {
            panic!("checked evaluation unexpectedly produced exact values")
        }
    }
}

fn evaluation_failure<T: core::fmt::Debug>(
    result: Result<T, CalculusFailure>,
) -> EvaluationFailure {
    match result {
        Err(CalculusFailure::Evaluation(failure)) => *failure,
        other => panic!("expected an evaluation failure: {other:?}"),
    }
}

#[test]
fn canonicalizes_bivariate_terms_and_preserves_evaluation() {
    let first = SurfacePolynomial::try_from_terms(vec![
        surface_term(0, 4, 4),
        surface_term(1, 1, 0),
        surface_term(-1, 1, 0),
        surface_term(2, 0, 1),
    ])
    .unwrap();
    let second = SurfacePolynomial::try_from_terms(vec![
        surface_term(2, 0, 1),
        surface_term(-1, 1, 0),
        surface_term(1, 1, 0),
        surface_term(0, 4, 4),
    ])
    .unwrap();
    let expected = SurfacePolynomial::try_from_terms(vec![surface_term(2, 0, 1)]).unwrap();
    assert_eq!(first.terms(), expected.terms());
    assert_eq!(second.terms(), expected.terms());

    let empty = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    let cancelled =
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0), surface_term(-1, 1, 0)])
            .unwrap();
    let cancelled_permuted =
        SurfacePolynomial::try_from_terms(vec![surface_term(-1, 1, 0), surface_term(1, 1, 0)])
            .unwrap();
    assert_eq!(cancelled.terms(), empty.terms());
    assert_eq!(cancelled_permuted.terms(), empty.terms());

    let evaluate = |x: SurfacePolynomial| {
        evaluate_surface(exact_input(x, constant(1), constant(1), constant(1))).unwrap()
    };
    assert_eq!(evaluate(first), evaluate(second));
    assert_eq!(
        evaluate(expected),
        SurfaceEvaluationProducer::Exact {
            x: algebraic(6),
            y: algebraic(1),
            z: algebraic(1),
        }
    );
}

#[test]
fn merges_duplicate_exponents_and_rejects_resource_excess() {
    let merged = SurfacePolynomial::try_from_terms(vec![
        surface_term(1, 1, 1),
        surface_term(-1, 1, 1),
        surface_term(3, 2, 0),
        surface_term(4, 2, 0),
    ])
    .unwrap();
    assert_eq!(
        merged.terms(),
        SurfacePolynomial::try_from_terms(vec![surface_term(7, 2, 0)])
            .unwrap()
            .terms()
    );

    let at_term_limit = (0..MAX_SURFACE_TERMS)
        .map(|index| surface_term(1, (index % 16) as u32, (index / 16) as u32))
        .collect();
    assert!(SurfacePolynomial::try_from_terms(at_term_limit).is_ok());
    let over_term_limit = (0..=MAX_SURFACE_TERMS)
        .map(|index| surface_term(1, (index % 17) as u32, (index / 17) as u32))
        .collect();
    assert_eq!(
        SurfacePolynomial::try_from_terms(over_term_limit),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceTermLimitExceeded {
                requested: MAX_SURFACE_TERMS + 1,
                maximum: MAX_SURFACE_TERMS,
            }
        )))
    );

    assert!(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, MAX_SURFACE_EXPONENT, 0)]).is_ok()
    );
    assert_eq!(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, MAX_SURFACE_EXPONENT + 1, 0)]),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceExponentLimitExceeded {
                requested: MAX_SURFACE_EXPONENT + 1,
                maximum: MAX_SURFACE_EXPONENT,
            }
        )))
    );
    assert_eq!(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, MAX_SURFACE_EXPONENT + 3)]),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceExponentLimitExceeded {
                requested: MAX_SURFACE_EXPONENT + 3,
                maximum: MAX_SURFACE_EXPONENT,
            }
        )))
    );
}

#[test]
fn evaluates_exact_surface_and_denominator_domain() {
    let surface = evaluate_surface(exact_input(
        polynomial(2, 1, 0),
        polynomial(3, 0, 1),
        constant(1),
        constant(1),
    ))
    .unwrap();
    assert_eq!(
        surface,
        SurfaceEvaluationProducer::Exact {
            x: algebraic(4),
            y: algebraic(9),
            z: algebraic(1),
        }
    );

    let empty = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    assert_eq!(
        evaluate_surface(exact_input(constant(1), constant(1), constant(1), empty)),
        Err(CalculusFailure::Rational(RationalFailure::UndefinedDomain))
    );

    let zero_at_point =
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0), surface_term(-2, 0, 0)])
            .unwrap();
    assert_eq!(
        evaluate_surface(exact_input(
            constant(1),
            constant(1),
            constant(1),
            zero_at_point
        )),
        Err(CalculusFailure::Rational(RationalFailure::UndefinedDomain))
    );

    let scaled = evaluate_surface(exact_input(
        polynomial(2, 1, 0),
        polynomial(2, 0, 1),
        constant(2),
        constant(2),
    ))
    .unwrap();
    assert_eq!(
        scaled,
        SurfaceEvaluationProducer::Exact {
            x: algebraic(2),
            y: algebraic(3),
            z: algebraic(1),
        }
    );
}

#[test]
fn evaluates_zero_coordinates_with_zero_exponents_as_identity() {
    let surface = evaluate_surface(SurfaceEvaluationInput {
        x: constant(1),
        y: polynomial(1, 1, 0),
        z: polynomial(1, 0, 1),
        denominator: constant(1),
        u: algebraic(0),
        v: algebraic(0),
    })
    .unwrap();
    assert_eq!(
        surface,
        SurfaceEvaluationProducer::Exact {
            x: algebraic(1),
            y: algebraic(0),
            z: algebraic(0),
        }
    );
}

#[test]
fn returns_checked_coordinates_with_provenance() {
    let coordinates = certified_coordinates(
        evaluate_surface_checked(
            checked_input(
                SurfacePolynomial::try_from_terms(vec![
                    surface_term(2, 1, 0),
                    surface_term(3, 0, 1),
                ])
                .unwrap(),
                constant(1),
                constant(1),
                constant(1),
                interval(1, 2),
                interval(2, 3),
                8,
            )
            .unwrap(),
        )
        .unwrap(),
    );

    assert!(coordinates.x().contains(&dyadic(8, 0)));
    assert!(coordinates.x().contains(&dyadic(13, 0)));
    assert!(coordinates.y().contains(&dyadic(1, 0)));
    assert!(coordinates.z().contains(&dyadic(1, 0)));
    assert_eq!(coordinates.owner(), OwnerId::new(1));
    assert_eq!(coordinates.requirement(), &requirement(8));
    assert_eq!(coordinates.source(), NecoImplementationSource::current());
}

#[test]
fn preserves_rational_surface_components() {
    let unscaled = certified_coordinates(
        evaluate_surface_checked(
            checked_input(
                polynomial(1, 1, 0),
                polynomial(1, 0, 1),
                constant(1),
                constant(1),
                interval(1, 2),
                interval(2, 3),
                8,
            )
            .unwrap(),
        )
        .unwrap(),
    );
    let scaled = certified_coordinates(
        evaluate_surface_checked(
            checked_input(
                polynomial(2, 1, 0),
                polynomial(2, 0, 1),
                constant(2),
                constant(2),
                interval(1, 2),
                interval(2, 3),
                8,
            )
            .unwrap(),
        )
        .unwrap(),
    );
    for enclosure in [unscaled.x(), scaled.x()] {
        assert!(enclosure.contains(&dyadic(1, 0)));
        assert!(enclosure.contains(&dyadic(2, 0)));
    }
    for enclosure in [unscaled.y(), scaled.y()] {
        assert!(enclosure.contains(&dyadic(2, 0)));
        assert!(enclosure.contains(&dyadic(3, 0)));
    }
    for enclosure in [unscaled.z(), scaled.z()] {
        assert!(enclosure.contains(&dyadic(1, 0)));
    }

    let first = evaluate_surface(exact_input(
        SurfacePolynomial::try_from_terms(vec![surface_term(2, 1, 0), surface_term(3, 0, 1)])
            .unwrap(),
        constant(1),
        constant(1),
        constant(1),
    ))
    .unwrap();
    let second = evaluate_surface(exact_input(
        SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1), surface_term(2, 1, 0)])
            .unwrap(),
        constant(1),
        constant(1),
        constant(1),
    ))
    .unwrap();
    assert_eq!(first, second);
}

#[test]
fn preserves_typed_failure_payloads() {
    let over_terms = (0..=MAX_SURFACE_TERMS)
        .map(|index| surface_term(1, (index % 18) as u32, (index / 18) as u32))
        .collect();
    assert_eq!(
        SurfacePolynomial::try_from_terms(over_terms),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceTermLimitExceeded {
                requested: MAX_SURFACE_TERMS + 1,
                maximum: MAX_SURFACE_TERMS,
            }
        )))
    );
    assert_eq!(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, MAX_SURFACE_EXPONENT + 5, 1)]),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceExponentLimitExceeded {
                requested: MAX_SURFACE_EXPONENT + 5,
                maximum: MAX_SURFACE_EXPONENT,
            }
        )))
    );

    let empty = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    assert!(matches!(
        evaluation_failure(evaluate_surface_checked(
            checked_input(
                constant(1),
                constant(1),
                constant(1),
                empty,
                interval(1, 2),
                interval(2, 3),
                8,
            )
            .unwrap()
        )),
        EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            witness: DomainWitness::ZeroDenominatorPolynomial,
            lower: RationalFailure::ZeroDenominatorPolynomial,
            ..
        }
    ));

    let crossing_zero =
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0), surface_term(-1, 0, 0)])
            .unwrap();
    assert!(matches!(
        evaluation_failure(evaluate_surface_checked(
            checked_input(
                constant(1),
                constant(1),
                constant(1),
                crossing_zero,
                interval(0, 2),
                interval(2, 3),
                8,
            )
            .unwrap()
        )),
        EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            witness: DomainWitness::DomainContainsZero { .. },
            lower: RationalFailure::DomainContainsZero,
            ..
        }
    ));

    for precision_bits in [0, MAX_PRECISION_BITS + 1] {
        assert!(matches!(
            evaluation_failure(checked_input(
                constant(1),
                constant(1),
                constant(1),
                constant(1),
                interval(1, 2),
                interval(2, 3),
                precision_bits,
            )),
            EvaluationFailure::PrecisionShortage {
                requested,
                operation: EvaluationOperation::SurfaceChecked,
                ..
            } if requested.get() == precision_bits
        ));
    }
}
