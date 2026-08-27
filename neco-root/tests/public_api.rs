use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_polynomial::{DerivativeOrder, Enclosure, Polynomial};
use neco_root::{
    classify_multiplicity, classify_region_sign, locate_membership_transitions, ConvergenceStatus,
    NewtonFallbackReason, RegionSign, RootBracket, RootEvidence, RootFailure, RootMultiplicity,
    RootRefinementMethod, RootResource, MAX_CANDIDATE_POINTS, MAX_DERIVATIVE_ORDER,
    MAX_ROOT_REFINEMENT_ITERATIONS, MAX_ROOT_REFINEMENT_PRECISION_BITS,
};

fn algebraic(value: usize) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i64) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), 0)
}

fn negative_two_plus_x_squared() -> Polynomial {
    let minus_two = algebraic(0).sub(&algebraic(2)).unwrap();
    Polynomial::from_coefficients(vec![minus_two, algebraic(0), algebraic(1)]).unwrap()
}

fn x_squared() -> Polynomial {
    Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(1)]).unwrap()
}

fn negative_x_squared() -> Polynomial {
    Polynomial::from_coefficients(vec![
        algebraic(0),
        algebraic(0),
        algebraic(0).sub(&algebraic(1)).unwrap(),
    ])
    .unwrap()
}

fn x_cubed() -> Polynomial {
    Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(0), algebraic(1)])
        .unwrap()
}

#[test]
fn accepts_only_sign_change_brackets() {
    let bracket = RootBracket::sign_change(
        negative_two_plus_x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        bracket.evidence(),
        RootEvidence::SignChange { .. }
    ));

    assert_eq!(
        RootBracket::sign_change(x_squared(), Enclosure::new(dyadic(-1), dyadic(1)).unwrap())
            .unwrap_err(),
        RootFailure::NoSignChange
    );
}

#[test]
fn classifies_even_multiplicity_touch() {
    let multiplicity =
        classify_multiplicity(&x_squared(), &dyadic(0), DerivativeOrder::new(3)).unwrap();
    assert_eq!(
        multiplicity,
        RootMultiplicity::Multiple(DerivativeOrder::new(2))
    );

    let bracket = RootBracket::even_multiplicity_touch(
        x_squared(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        dyadic(0),
        DerivativeOrder::new(3),
    )
    .unwrap();
    assert!(matches!(
        bracket.evidence(),
        RootEvidence::EvenMultiplicityTouch { .. }
    ));
    let isolation = bracket.refine_bisection(4).unwrap();
    assert_eq!(
        isolation.witness().status(),
        ConvergenceStatus::ExactDyadicRoot
    );
    assert_eq!(isolation.witness().iterations(), 0);
    assert_eq!(isolation.enclosure().lower(), &dyadic(0));
    assert_eq!(isolation.enclosure().upper(), &dyadic(0));
}

#[test]
fn refines_sign_change_by_bisection() {
    let bracket = RootBracket::sign_change(
        negative_two_plus_x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
    )
    .unwrap();
    let isolation = bracket.refine_bisection(4).unwrap();
    assert_eq!(
        isolation.witness().method(),
        RootRefinementMethod::Bisection
    );
    assert_eq!(isolation.witness().status(), ConvergenceStatus::Refined);
    assert_eq!(isolation.witness().iterations(), 4);
    assert!(isolation.enclosure().lower() >= &dyadic(1));
    assert!(isolation.enclosure().upper() <= &dyadic(2));
    assert!(isolation.enclosure().width().unwrap() < dyadic(1));
}

#[test]
fn refines_sign_change_by_newton_bisection() {
    let bracket = RootBracket::sign_change(
        negative_two_plus_x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
    )
    .unwrap();
    let isolation = bracket.refine_newton_bisection(4, 8).unwrap();
    assert_eq!(
        isolation.witness().method(),
        RootRefinementMethod::NewtonBisection
    );
    assert_eq!(isolation.witness().status(), ConvergenceStatus::Refined);
    assert_eq!(isolation.witness().iterations(), 4);
    assert!(isolation.enclosure().lower() >= &dyadic(1));
    assert!(isolation.enclosure().upper() <= &dyadic(2));
}

#[test]
fn newton_derivative_zero_fallback_is_observable() {
    let bracket =
        RootBracket::sign_change(x_cubed(), Enclosure::new(dyadic(-1), dyadic(1)).unwrap())
            .unwrap();
    let isolation = bracket.refine_newton_bisection(1, 8).unwrap();

    assert_eq!(isolation.witness().newton_steps(), 0);
    assert_eq!(isolation.witness().newton_fallback_count(), 1);
    assert_eq!(
        isolation.witness().newton_fallback_reason(),
        Some(NewtonFallbackReason::DerivativeEnclosureContainsZero)
    );
    assert_eq!(
        isolation.witness().status(),
        ConvergenceStatus::ExactDyadicRoot
    );
}

#[test]
fn classifies_constant_zero_region_sign() {
    let polynomial = Polynomial::constant(algebraic(0)).unwrap();
    let request = neco_root::MembershipJudgmentRequest::new(
        polynomial,
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        Vec::new(),
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ConstantZero
    );
}

#[test]
fn classifies_region_sign_with_candidate_outside_enclosure_as_error() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(0), dyadic(1)).unwrap(),
        vec![dyadic(2)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap_err(),
        RootFailure::PointOutsideEnclosure
    );
}

#[test]
fn interval_zero_exclusion_proves_nonconstant_positive_sign() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
        Vec::new(),
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ConstantPositive
    );
}

#[test]
fn interval_zero_exclusion_proves_nonconstant_negative_sign() {
    let request = neco_root::MembershipJudgmentRequest::new(
        negative_x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
        Vec::new(),
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ConstantNegative
    );
}

#[test]
fn classifies_constant_negative_region_sign() {
    let negative = algebraic(0).sub(&algebraic(1)).unwrap();
    let request = neco_root::MembershipJudgmentRequest::new(
        Polynomial::constant(negative).unwrap(),
        Enclosure::new(dyadic(-1), dyadic(2)).unwrap(),
        Vec::new(),
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ConstantNegative
    );
}

#[test]
fn locate_membership_transitions_marks_simple_root_as_transition() {
    let minus_one = algebraic(0).sub(&algebraic(1)).unwrap();
    let request = neco_root::MembershipJudgmentRequest::new(
        Polynomial::from_coefficients(vec![minus_one, algebraic(0), algebraic(1)]).unwrap(),
        Enclosure::new(dyadic(-2), dyadic(2)).unwrap(),
        vec![dyadic(-1), dyadic(1)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();
    let report = locate_membership_transitions(&request).unwrap();

    assert_eq!(report.loci().len(), 2);
    assert_eq!(report.loci()[0].point_index(), 0);
    assert_eq!(
        report.loci()[0].kind(),
        neco_root::MembershipLocusKind::Transition
    );
    assert_eq!(report.loci()[0].multiplicity(), RootMultiplicity::Simple);
    assert_eq!(report.loci()[1].point_index(), 1);
    assert_eq!(
        report.loci()[1].kind(),
        neco_root::MembershipLocusKind::Transition
    );
    assert_eq!(report.loci()[1].multiplicity(), RootMultiplicity::Simple);
}

#[test]
fn locate_membership_transitions_marks_even_multiplicity_as_singular_contact() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        vec![dyadic(0)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();
    let report = locate_membership_transitions(&request).unwrap();

    assert_eq!(report.loci().len(), 1);
    assert_eq!(report.loci()[0].point_index(), 0);
    assert_eq!(
        report.loci()[0].kind(),
        neco_root::MembershipLocusKind::SingularContact
    );
    assert_eq!(
        report.loci()[0].multiplicity(),
        RootMultiplicity::Multiple(DerivativeOrder::new(2))
    );
}

#[test]
fn even_multiplicity_root_reports_zero_without_claiming_sign_change() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        vec![dyadic(0)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();
    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ContainsZero
    );
}

#[test]
fn endpoint_root_reports_zero_without_claiming_interval_sign_change() {
    let polynomial = Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap();
    let request = neco_root::MembershipJudgmentRequest::new(
        polynomial,
        Enclosure::new(dyadic(0), dyadic(1)).unwrap(),
        vec![dyadic(0)],
        DerivativeOrder::new(2),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::ContainsZero
    );
}

#[test]
fn interior_odd_multiplicity_root_reports_sign_variation() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_cubed(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        vec![dyadic(0)],
        DerivativeOrder::new(4),
        64,
    )
    .unwrap();

    assert_eq!(
        classify_region_sign(&request).unwrap().sign(),
        RegionSign::SignVaries
    );
}

#[test]
fn locate_membership_transitions_empty_candidates_yield_empty_report() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        Vec::new(),
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();
    let report = locate_membership_transitions(&request).unwrap();
    assert_eq!(report.loci().len(), 0);
}

#[test]
fn locate_membership_transitions_with_candidate_outside_enclosure_as_error() {
    let request = neco_root::MembershipJudgmentRequest::new(
        x_squared(),
        Enclosure::new(dyadic(0), dyadic(1)).unwrap(),
        vec![dyadic(2)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(
        locate_membership_transitions(&request).unwrap_err(),
        RootFailure::PointOutsideEnclosure
    );
}

#[test]
fn membership_judgment_request_accessors_preserve_construction_values() {
    let polynomial = x_squared();
    let enclosure = Enclosure::new(dyadic(-1), dyadic(1)).unwrap();
    let request = neco_root::MembershipJudgmentRequest::new(
        polynomial,
        enclosure,
        vec![dyadic(0)],
        DerivativeOrder::new(3),
        64,
    )
    .unwrap();

    assert_eq!(request.target_polynomial(), &x_squared());
    assert_eq!(
        request.target_enclosure(),
        &Enclosure::new(dyadic(-1), dyadic(1)).unwrap()
    );
    assert_eq!(request.candidate_points(), &[dyadic(0)]);
    assert_eq!(request.max_order(), DerivativeOrder::new(3));
    assert_eq!(request.zero_exclusion_precision_bits(), 64);
}

#[test]
fn rejects_public_resource_requests_before_refinement() {
    for candidate_count in [1, 16, MAX_CANDIDATE_POINTS - 1, MAX_CANDIDATE_POINTS] {
        let request = neco_root::MembershipJudgmentRequest::new(
            x_squared(),
            Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
            (0..candidate_count).map(|_| dyadic(1)).collect(),
            DerivativeOrder::new(2),
            64,
        )
        .unwrap();
        assert_eq!(
            neco_root::classify_region_sign(&request).unwrap().sign(),
            RegionSign::Undetermined
        );
    }
    assert_eq!(
        neco_root::MembershipJudgmentRequest::new(
            x_squared(),
            Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
            (0..MAX_CANDIDATE_POINTS + 1).map(|_| dyadic(0)).collect(),
            DerivativeOrder::new(3),
            64,
        ),
        Err(RootFailure::ResourceLimit {
            resource: RootResource::CandidatePoints,
            requested: MAX_CANDIDATE_POINTS + 1,
            maximum: MAX_CANDIDATE_POINTS,
        })
    );
    for derivative_order in [1, 8, MAX_DERIVATIVE_ORDER - 1, MAX_DERIVATIVE_ORDER] {
        let request = neco_root::MembershipJudgmentRequest::new(
            x_squared(),
            Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
            Vec::new(),
            DerivativeOrder::new(derivative_order),
            64,
        )
        .unwrap();
        assert!(neco_root::classify_region_sign(&request).is_ok());
    }
    assert_eq!(
        neco_root::MembershipJudgmentRequest::new(
            x_squared(),
            Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
            Vec::new(),
            DerivativeOrder::new(3),
            MAX_ROOT_REFINEMENT_PRECISION_BITS + 1,
        ),
        Err(RootFailure::ResourceLimit {
            resource: RootResource::RegionSignPrecisionBits,
            requested: (MAX_ROOT_REFINEMENT_PRECISION_BITS + 1) as usize,
            maximum: MAX_ROOT_REFINEMENT_PRECISION_BITS as usize,
        })
    );
    assert_eq!(
        neco_root::MembershipJudgmentRequest::new(
            x_squared(),
            Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
            Vec::new(),
            DerivativeOrder::new(MAX_DERIVATIVE_ORDER + 1),
            64,
        ),
        Err(RootFailure::ResourceLimit {
            resource: RootResource::DerivativeOrder,
            requested: MAX_DERIVATIVE_ORDER + 1,
            maximum: MAX_DERIVATIVE_ORDER,
        })
    );
    let bracket = RootBracket::sign_change(
        negative_two_plus_x_squared(),
        Enclosure::new(dyadic(1), dyadic(2)).unwrap(),
    )
    .unwrap();
    for iterations in [
        1,
        8,
        MAX_ROOT_REFINEMENT_ITERATIONS - 1,
        MAX_ROOT_REFINEMENT_ITERATIONS,
    ] {
        assert!(bracket.refine_bisection(iterations).is_ok());
    }
    assert_eq!(
        bracket.refine_bisection(MAX_ROOT_REFINEMENT_ITERATIONS + 1),
        Err(RootFailure::ResourceLimit {
            resource: RootResource::RefinementIterations,
            requested: MAX_ROOT_REFINEMENT_ITERATIONS + 1,
            maximum: MAX_ROOT_REFINEMENT_ITERATIONS,
        })
    );
    assert_eq!(
        bracket.refine_newton_bisection(1, MAX_ROOT_REFINEMENT_PRECISION_BITS + 1),
        Err(RootFailure::ResourceLimit {
            resource: RootResource::RefinementPrecisionBits,
            requested: (MAX_ROOT_REFINEMENT_PRECISION_BITS + 1) as usize,
            maximum: MAX_ROOT_REFINEMENT_PRECISION_BITS as usize,
        })
    );
    let exact = RootBracket::even_multiplicity_touch(
        x_squared(),
        Enclosure::new(dyadic(-1), dyadic(1)).unwrap(),
        dyadic(0),
        DerivativeOrder::new(2),
    )
    .unwrap();
    for precision_bits in [
        1,
        64,
        MAX_ROOT_REFINEMENT_PRECISION_BITS - 1,
        MAX_ROOT_REFINEMENT_PRECISION_BITS,
    ] {
        assert!(exact.refine_newton_bisection(1, precision_bits).is_ok());
    }
}
