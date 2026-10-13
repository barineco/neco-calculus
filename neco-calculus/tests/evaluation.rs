use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_bigint::{BigInt, BigUint, BigintError, Dyadic, RawRational};
use neco_calculus::{
    construct_transcendental_evaluation, evaluate_implicit, evaluate_rational_polynomial,
    evaluate_surface, evaluate_surface_checked, jet_at, CalculusFailure, DerivativeOrder,
    DomainWitness, Enclosure, EvaluationFailure, EvaluationInputSnapshot, EvaluationOperation,
    ImplicitEvaluationInput, ImplicitEvaluationProducer, ImplicitPolynomial3, ImplicitTerm, Jet,
    JetFailure, JetSource, OwnerId, Polynomial, PolynomialResourceFailure, PrecisionWitness,
    RationalFailure, RationalPolynomialInput, RationalSurfacePolynomials, SurfaceEvaluationInput,
    SurfaceEvaluationInput2, SurfaceEvaluationProducer, SurfacePolynomial, SurfaceTerm,
    TranscendentalEvaluationCandidate, TranscendentalEvaluationConstructionInput,
    TranscendentalEvaluationKind, TranscendentalEvaluationRequest, MAX_COEFFICIENTS,
    MAX_IMPLICIT_TERMS, MAX_JET_ORDER, MAX_PRECISION_BITS, MAX_SURFACE_EXPONENT, MAX_SURFACE_TERMS,
};
use neco_expr::{
    AbsoluteBits, ConsumerId, ExactDecisionRequest, ExactExpressionRequirement, ExprId,
    NecoImplementationSource, PrecisionFailurePayload,
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

fn implicit_term(coefficient: i32, exponent_x: u32) -> ImplicitTerm {
    ImplicitTerm {
        coefficient: algebraic(coefficient),
        exponent_x,
        exponent_y: 0,
        exponent_z: 0,
    }
}

fn interval(lower: i32, upper: i32) -> Enclosure {
    Enclosure::new(dyadic(lower, 0), dyadic(upper, 0)).unwrap()
}

fn requirement(consumer: u32, expression: u32, precision: u32) -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(consumer),
        &[ExprId::new(expression)],
        ExactDecisionRequest::Zero(ExprId::new(expression)),
        AbsoluteBits::new(precision),
    )
    .unwrap()
}

fn source() -> NecoImplementationSource {
    NecoImplementationSource::current()
}

fn owner() -> OwnerId {
    OwnerId::new(1)
}

fn checked_input(
    x: SurfacePolynomial,
    y: SurfacePolynomial,
    z: SurfacePolynomial,
    denominator: SurfacePolynomial,
    precision_bits: u32,
) -> Result<SurfaceEvaluationInput2, CalculusFailure> {
    SurfaceEvaluationInput2::new(
        RationalSurfacePolynomials {
            x,
            y,
            z,
            denominator,
        },
        interval(1, 2),
        interval(2, 3),
        owner(),
        requirement(11, 500, precision_bits),
        source(),
    )
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
fn surface_polynomials_normalize_coefficients_and_term_order() {
    let normalized = SurfacePolynomial::try_from_terms(vec![
        surface_term(0, 9, 9),
        surface_term(1, 1, 0),
        surface_term(-1, 1, 0),
        surface_term(1, 0, 1),
        surface_term(2, 0, 1),
        surface_term(4, 2, 0),
    ])
    .unwrap();
    let expected =
        SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1), surface_term(4, 2, 0)])
            .unwrap();
    let permutation = SurfacePolynomial::try_from_terms(vec![
        surface_term(4, 2, 0),
        surface_term(2, 0, 1),
        surface_term(-1, 1, 0),
        surface_term(1, 0, 1),
        surface_term(0, 9, 9),
        surface_term(1, 1, 0),
    ])
    .unwrap();

    assert_eq!(normalized.terms(), expected.terms());
    assert_eq!(permutation.terms(), expected.terms());
}

#[test]
fn term_order_is_canonical_and_evaluation_is_deterministic() {
    let first =
        SurfacePolynomial::try_from_terms(vec![surface_term(2, 1, 0), surface_term(3, 0, 1)])
            .unwrap();
    let second =
        SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1), surface_term(2, 1, 0)])
            .unwrap();
    assert_eq!(first.terms(), second.terms());

    let constant = || SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap();
    let evaluate = |x| {
        evaluate_surface(SurfaceEvaluationInput {
            x,
            y: constant(),
            z: constant(),
            denominator: constant(),
            u: algebraic(2),
            v: algebraic(3),
        })
        .unwrap()
    };
    assert_eq!(evaluate(first), evaluate(second));
}

#[test]
fn term_sets_reject_zero_duplicate_and_excess_inputs() {
    assert!(SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).is_ok());
    assert!(ImplicitPolynomial3::try_from_terms(vec![implicit_term(1, 0)]).is_ok());
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(vec![implicit_term(0, 0)]),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::ZeroImplicitTerm
        )))
    );
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(vec![implicit_term(1, 2), implicit_term(2, 2)]),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::DuplicateImplicitExponent
        )))
    );
    let surface_middle = (0..16).map(|index| surface_term(1, index, 0)).collect();
    assert!(SurfacePolynomial::try_from_terms(surface_middle).is_ok());
    let implicit_middle = (0..16).map(|index| implicit_term(1, index)).collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_middle).is_ok());

    let surface_pair = |index: usize| -> SurfaceTerm {
        let width = (MAX_SURFACE_EXPONENT as usize) + 1;
        surface_term(1, (index % width) as u32, (index / width) as u32)
    };
    let surface_near_limit = (0..MAX_SURFACE_TERMS - 1).map(surface_pair).collect();
    assert!(SurfacePolynomial::try_from_terms(surface_near_limit).is_ok());
    let surface_at_limit = (0..MAX_SURFACE_TERMS).map(surface_pair).collect();
    assert!(SurfacePolynomial::try_from_terms(surface_at_limit).is_ok());
    let surface_terms = (0..=MAX_SURFACE_TERMS).map(surface_pair).collect();
    assert_eq!(
        SurfacePolynomial::try_from_terms(surface_terms),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::SurfaceTermLimitExceeded {
                requested: MAX_SURFACE_TERMS + 1,
                maximum: MAX_SURFACE_TERMS,
            }
        )))
    );

    let implicit_near_limit = (0..MAX_IMPLICIT_TERMS - 1)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_near_limit).is_ok());
    let implicit_at_limit = (0..MAX_IMPLICIT_TERMS)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_at_limit).is_ok());
    let implicit_terms = (0..=MAX_IMPLICIT_TERMS)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(implicit_terms),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::ImplicitTermLimitExceeded {
                requested: MAX_IMPLICIT_TERMS + 1,
                maximum: MAX_IMPLICIT_TERMS,
            }
        )))
    );
}

#[test]
fn exact_surface_and_implicit_evaluation_preserve_values() {
    let surface = evaluate_surface(SurfaceEvaluationInput {
        x: SurfacePolynomial::try_from_terms(vec![surface_term(2, 1, 0)]).unwrap(),
        y: SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1)]).unwrap(),
        z: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        denominator: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        u: algebraic(2),
        v: algebraic(3),
    })
    .unwrap();
    match surface {
        SurfaceEvaluationProducer::Exact { x, y, z } => {
            assert_eq!(x, algebraic(4));
            assert_eq!(y, algebraic(9));
            assert_eq!(z, algebraic(1));
        }
        SurfaceEvaluationProducer::Certified { .. } => {
            panic!("exact evaluation unexpectedly produced a certified enclosure")
        }
    }

    let implicit = evaluate_implicit(ImplicitEvaluationInput {
        equation: ImplicitPolynomial3::try_from_terms(vec![implicit_term(2, 1)]).unwrap(),
        x: algebraic(3),
        y: algebraic(1),
        z: algebraic(1),
    })
    .unwrap();
    assert_eq!(
        implicit,
        ImplicitEvaluationProducer::Exact {
            scalar: algebraic(6)
        }
    );
}

#[test]
fn checked_surface_result_carries_requirement_lineage() {
    let constant = || SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap();
    let result = evaluate_surface_checked(
        checked_input(
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0)]).unwrap(),
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 1)]).unwrap(),
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 1)]).unwrap(),
            constant(),
            8,
        )
        .unwrap(),
    )
    .unwrap();

    match result {
        SurfaceEvaluationProducer::Certified { coordinates } => {
            assert!(coordinates.x().contains(&dyadic(1, 0)));
            assert!(coordinates.x().contains(&dyadic(2, 0)));
            assert!(coordinates.y().contains(&dyadic(2, 0)));
            assert!(coordinates.y().contains(&dyadic(3, 0)));
            assert!(coordinates.z().contains(&dyadic(2, 0)));
            assert!(coordinates.z().contains(&dyadic(6, 0)));
            assert_eq!(coordinates.owner(), owner());
            assert_eq!(coordinates.requirement(), &requirement(11, 500, 8));
            assert_eq!(coordinates.source(), source());
        }
        SurfaceEvaluationProducer::Exact { .. } => {
            panic!("checked evaluation unexpectedly produced an exact value")
        }
    }
}

#[test]
fn checked_surface_evaluation_rejects_invalid_denominators_and_precisions() {
    let constant = || SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap();
    let empty = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    assert!(matches!(
        evaluation_failure(evaluate_surface_checked(
            checked_input(constant(), constant(), constant(), empty, 8).unwrap()
        )),
        EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            witness: DomainWitness::ZeroDenominatorPolynomial,
            lower: RationalFailure::ZeroDenominatorPolynomial,
            ..
        }
    ));

    let denominator =
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0), surface_term(-1, 0, 0)])
            .unwrap();
    let input = SurfaceEvaluationInput2::new(
        RationalSurfacePolynomials {
            x: constant(),
            y: constant(),
            z: constant(),
            denominator,
        },
        interval(0, 2),
        interval(2, 3),
        owner(),
        requirement(11, 500, 8),
        source(),
    )
    .unwrap();
    assert!(matches!(
        evaluation_failure(evaluate_surface_checked(input)),
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
                constant(),
                constant(),
                constant(),
                constant(),
                precision_bits,
            )),
            EvaluationFailure::PrecisionShortage {
                operation: EvaluationOperation::SurfaceChecked,
                requested,
                witness: neco_calculus::PrecisionWitness::Limit { maximum },
                ..
            } if requested.get() == precision_bits && maximum == MAX_PRECISION_BITS
        ));
    }
}

#[test]
fn wrong_expression_is_rejected_with_expected_and_actual_provenance() {
    let result =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                achieved: interval(0, 1),
                owner: owner(),
                requirement: requirement(11, 501, 8),
                source: source(),
            },
            TranscendentalEvaluationRequest::new(owner(), requirement(11, 500, 8), source()),
        ));
    match evaluation_failure(result) {
        EvaluationFailure::Lower {
            operation: EvaluationOperation::Transcendental,
            lower:
                neco_calculus::LowerEvaluationFailure::Expression(
                    neco_expr::NecoFailure::ProvenanceMismatch { location, payload },
                ),
            ..
        } => {
            assert_eq!(location.expr(), Some(ExprId::new(501)));
            assert_eq!(payload.expected().expr_id(), Some(ExprId::new(500)));
            assert_eq!(payload.actual().expr_id(), Some(ExprId::new(501)));
        }
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn wrong_consumer_is_rejected_with_expected_and_actual_provenance() {
    let result =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                achieved: interval(0, 1),
                owner: owner(),
                requirement: requirement(12, 500, 8),
                source: source(),
            },
            TranscendentalEvaluationRequest::new(owner(), requirement(11, 500, 8), source()),
        ));
    match evaluation_failure(result) {
        EvaluationFailure::Lower {
            lower:
                neco_calculus::LowerEvaluationFailure::Expression(
                    neco_expr::NecoFailure::ProvenanceMismatch { payload, .. },
                ),
            ..
        } => {
            assert_eq!(payload.expected().consumer_id(), Some(ConsumerId::new(11)));
            assert_eq!(payload.actual().consumer_id(), Some(ConsumerId::new(12)));
        }
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn domain_violation_preserves_surface_input_and_zero_witness() {
    let denominator =
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 1, 0), surface_term(-1, 0, 0)])
            .unwrap();
    let result = evaluate_surface_checked(
        SurfaceEvaluationInput2::new(
            RationalSurfacePolynomials {
                x: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
                y: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
                z: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
                denominator,
            },
            interval(0, 2),
            interval(2, 3),
            owner(),
            requirement(11, 500, 8),
            source(),
        )
        .unwrap(),
    );
    match evaluation_failure(result) {
        EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
            witness: DomainWitness::DomainContainsZero { denominator },
            lower: RationalFailure::DomainContainsZero,
            ..
        } => {
            assert_eq!(surface.x.terms().len(), 1);
            assert!(u.contains(&dyadic(0, 0)));
            assert!(v.contains(&dyadic(2, 0)));
            assert!(denominator.contains(&dyadic(0, 0)));
        }
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn zero_denominator_polynomial_preserves_original_input() {
    let empty = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    let result = evaluate_surface_checked(
        checked_input(
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
            SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
            empty,
            8,
        )
        .unwrap(),
    );
    match evaluation_failure(result) {
        EvaluationFailure::DomainViolation {
            input: EvaluationInputSnapshot::SurfaceChecked { surface, .. },
            witness: DomainWitness::ZeroDenominatorPolynomial,
            lower: RationalFailure::ZeroDenominatorPolynomial,
            ..
        } => assert!(surface.denominator.terms().is_empty()),
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn precision_shortage_preserves_requirement_and_lower_precision_failure() {
    let result = checked_input(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        257,
    );
    match evaluation_failure(result) {
        EvaluationFailure::PrecisionShortage {
            operation,
            owner: result_owner,
            requirement: result_requirement,
            source: result_source,
            input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
            requested,
            witness: PrecisionWitness::Limit { maximum },
            lower:
                neco_calculus::PrecisionShortageCause::RequestedBeyondLimit {
                    requested: cause_requested,
                    maximum: cause_maximum,
                },
        } => {
            assert_eq!(operation, EvaluationOperation::SurfaceChecked);
            assert_eq!(result_owner, owner());
            assert_eq!(result_source, source());
            assert_eq!(requested, AbsoluteBits::new(257));
            assert_eq!(cause_requested, AbsoluteBits::new(257));
            assert_eq!(cause_maximum, MAX_PRECISION_BITS);
            assert_eq!(result_requirement, requirement(11, 500, 257));
            assert_eq!(maximum, MAX_PRECISION_BITS);
            assert_eq!(surface.x.terms().len(), 1);
            assert!(u.contains(&dyadic(1, 0)));
            assert!(v.contains(&dyadic(2, 0)));
        }
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn lower_failure_passthrough_preserves_operation_input_and_payload() {
    let overflowing_x = SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(1),
        exponent_u: 2,
        exponent_v: 0,
    }])
    .unwrap();
    let huge_u = Enclosure::new(dyadic(1, u32::MAX), dyadic(1, u32::MAX)).unwrap();
    let result = evaluate_surface_checked(
        SurfaceEvaluationInput2::new(
            RationalSurfacePolynomials {
                x: overflowing_x,
                y: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
                z: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
                denominator: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)])
                    .unwrap(),
            },
            huge_u,
            interval(2, 3),
            owner(),
            requirement(11, 500, 8),
            source(),
        )
        .unwrap(),
    );
    match evaluation_failure(result) {
        EvaluationFailure::Lower {
            operation,
            owner: result_owner,
            requirement: result_requirement,
            source: result_source,
            input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
            witness,
            lower: neco_calculus::LowerEvaluationFailure::Calculus(lower),
        } => {
            assert_eq!(operation, EvaluationOperation::SurfaceChecked);
            assert_eq!(witness.operation(), EvaluationOperation::SurfaceChecked);
            assert_eq!(result_owner, owner());
            assert_eq!(result_requirement, requirement(11, 500, 8));
            assert_eq!(result_source, source());
            assert_eq!(surface.x.terms().len(), 1);
            assert_eq!(surface.x.terms()[0].exponent_u, 2);
            assert!(u.contains(&dyadic(1, u32::MAX)));
            assert!(v.contains(&dyadic(2, 0)));
            let required = BigUint::try_from(2_u64 * u64::from(u32::MAX)).unwrap();
            assert_eq!(
                *lower,
                CalculusFailure::Arithmetic(BigintError::ExponentOverflow {
                    required,
                    maximum: u32::MAX,
                })
            );
        }
        other => panic!("unexpected typed result: {other:?}"),
    }
}

#[test]
fn transcendental_result_and_undetermined_keep_requirement() {
    let enclosure = Enclosure::new(dyadic(0, 8), dyadic(1, 8)).unwrap();
    let request = TranscendentalEvaluationRequest::new(owner(), requirement(21, 7, 8), source());
    let certified =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Enclosure {
                enclosure,
                owner: owner(),
                requirement: requirement(21, 7, 8),
                source: source(),
            },
            request,
        ))
        .unwrap();
    assert_eq!(certified.requirement(), &requirement(21, 7, 8));
    let undetermined =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                achieved: interval(0, 1),
                owner: owner(),
                requirement: requirement(21, 7, 8),
                source: source(),
            },
            TranscendentalEvaluationRequest::new(owner(), requirement(21, 7, 8), source()),
        ))
        .unwrap();
    assert!(undetermined.is_undetermined());
    assert_eq!(undetermined.requirement(), &requirement(21, 7, 8));
}

#[test]
fn rational_polynomial_evaluation_preserves_exact_value() {
    let polynomial = RationalPolynomial::from_coefficients(vec![rational(1), rational(2)]);
    let result = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial,
        point: algebraic(3),
    })
    .unwrap();
    assert_eq!(result.value(), &algebraic(7));
}

#[test]
fn facade_resource_limits_reject_before_conversion_or_derivation() {
    for coefficient_count in [1, 16, MAX_COEFFICIENTS - 1, MAX_COEFFICIENTS] {
        let polynomial = RationalPolynomial::from_coefficients(
            (0..coefficient_count).map(|_| rational(1)).collect(),
        );
        assert!(evaluate_rational_polynomial(RationalPolynomialInput {
            polynomial,
            point: algebraic(1),
        })
        .is_ok());
    }
    let coefficients = (0..=neco_calculus::MAX_COEFFICIENTS)
        .map(|_| rational(1))
        .collect();
    let polynomial = RationalPolynomial::from_coefficients(coefficients);
    assert_eq!(
        evaluate_rational_polynomial(RationalPolynomialInput {
            polynomial,
            point: algebraic(1),
        }),
        Err(CalculusFailure::Polynomial(
            neco_calculus::PolynomialFailure::Resource(
                PolynomialResourceFailure::CoefficientLimitExceeded {
                    requested: neco_calculus::MAX_COEFFICIENTS + 1,
                    maximum: neco_calculus::MAX_COEFFICIENTS,
                }
            )
        ))
    );

    let polynomial = Polynomial::constant(algebraic(1)).unwrap();
    for order in [1, 8, MAX_JET_ORDER - 1, MAX_JET_ORDER] {
        assert!(jet_at(&polynomial, &algebraic(0), DerivativeOrder::new(order),).is_ok());
    }
    assert_eq!(
        jet_at(
            &polynomial,
            &algebraic(0),
            DerivativeOrder::new(neco_calculus::MAX_JET_ORDER + 1),
        ),
        Err(CalculusFailure::Jet(JetFailure::OrderLimitExceeded {
            requested: neco_calculus::MAX_JET_ORDER + 1,
            maximum: neco_calculus::MAX_JET_ORDER,
        }))
    );

    struct MustNotEvaluate;
    impl JetSource for MustNotEvaluate {
        fn jet_at(
            &self,
            _point: &RealAlgebraic,
            _order: DerivativeOrder,
        ) -> Result<Jet, CalculusFailure> {
            panic!("resource validation must precede source evaluation")
        }
    }
    assert_eq!(
        jet_at(
            &MustNotEvaluate,
            &algebraic(0),
            DerivativeOrder::new(neco_calculus::MAX_JET_ORDER + 1),
        ),
        Err(CalculusFailure::Jet(JetFailure::OrderLimitExceeded {
            requested: neco_calculus::MAX_JET_ORDER + 1,
            maximum: neco_calculus::MAX_JET_ORDER,
        }))
    );
}

#[test]
fn transcendental_connector_checks_expression_precision_and_width() {
    let request = |expression: u32, consumer: u32, precision: u32| {
        TranscendentalEvaluationRequest::new(
            owner(),
            requirement(consumer, expression, precision),
            source(),
        )
    };
    let candidate_exact = |expression: u32, consumer: u32, precision: u32| {
        TranscendentalEvaluationCandidate::ExactAlgebraic {
            value: algebraic(1),
            owner: owner(),
            requirement: requirement(consumer, expression, precision),
            source: source(),
        }
    };
    for precision_bits in [1, 64, MAX_PRECISION_BITS - 1, MAX_PRECISION_BITS] {
        assert!(construct_transcendental_evaluation(
            TranscendentalEvaluationConstructionInput::new(
                candidate_exact(7, 21, precision_bits),
                request(7, 21, precision_bits),
            )
        )
        .is_ok());
    }
    let narrow = Enclosure::new(dyadic(0, 8), dyadic(1, 8)).unwrap();
    let result =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Enclosure {
                enclosure: narrow,
                owner: owner(),
                requirement: requirement(21, 7, 8),
                source: source(),
            },
            request(7, 21, 8),
        ))
        .unwrap();
    assert_eq!(result.kind(), TranscendentalEvaluationKind::Enclosure);
    assert_eq!(
        result.as_enclosure().unwrap().width().unwrap(),
        dyadic(1, 8)
    );
    assert_eq!(result.owner(), owner());
    assert_eq!(result.requirement(), &requirement(21, 7, 8));
    assert_eq!(result.source(), source());

    let wide = Enclosure::new(dyadic(0, 0), dyadic(1, 0)).unwrap();
    let wide_result =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Enclosure {
                enclosure: wide,
                owner: owner(),
                requirement: requirement(21, 7, 8),
                source: source(),
            },
            request(7, 21, 8),
        ));
    match evaluation_failure(wide_result) {
        EvaluationFailure::PrecisionShortage {
            operation: neco_calculus::EvaluationOperation::Transcendental,
            requested,
            witness:
                neco_calculus::PrecisionWitness::Enclosure {
                    enclosure: observed,
                },
            lower:
                neco_calculus::PrecisionShortageCause::Exhausted(
                    neco_expr::NecoFailure::PrecisionExhaustion {
                        requested: cause_requested,
                        payload,
                        ..
                    },
                ),
            ..
        } => {
            let PrecisionFailurePayload::EnclosureWidth { actual_width } = *payload else {
                panic!("unexpected precision payload: {payload:?}");
            };
            assert_eq!(requested.get(), 8);
            assert_eq!(cause_requested.get(), 8);
            assert!(observed.contains(&dyadic(0, 0)));
            assert!(observed.contains(&dyadic(1, 0)));
            assert_eq!(actual_width, dyadic(1, 0));
        }
        other => panic!("unexpected typed result: {other:?}"),
    }

    for precision_bits in [0, MAX_PRECISION_BITS + 1] {
        assert!(matches!(
            evaluation_failure(construct_transcendental_evaluation(
                TranscendentalEvaluationConstructionInput::new(
                    TranscendentalEvaluationCandidate::Undetermined {
                        achieved: interval(0, 1),
                        owner: owner(),
                        requirement: requirement(21, 7, precision_bits),
                        source: source(),
                    },
                    request(7, 21, precision_bits),
                )
            )),
            EvaluationFailure::PrecisionShortage { requested, .. }
                if requested.get() == precision_bits
        ));
    }

    let undetermined =
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                achieved: interval(0, 1),
                owner: owner(),
                requirement: requirement(21, 7, 8),
                source: source(),
            },
            request(7, 21, 8),
        ))
        .unwrap();
    assert!(undetermined.is_undetermined());
    assert_eq!(undetermined.requirement(), &requirement(21, 7, 8));

    assert!(matches!(
        evaluation_failure(construct_transcendental_evaluation(
            TranscendentalEvaluationConstructionInput::new(
                TranscendentalEvaluationCandidate::Undetermined {
                    achieved: interval(0, 1),
                    owner: owner(),
                    requirement: requirement(21, 8, 8),
                    source: source(),
                },
                request(7, 21, 8),
            )
        )),
        EvaluationFailure::Lower {
            lower: neco_calculus::LowerEvaluationFailure::Expression(
                neco_expr::NecoFailure::ProvenanceMismatch { location, payload }
            ),
            ..
        } if location.expr() == Some(ExprId::new(8))
            && payload.expected().expr_id() == Some(ExprId::new(7))
            && payload.actual().expr_id() == Some(ExprId::new(8))
    ));
}
