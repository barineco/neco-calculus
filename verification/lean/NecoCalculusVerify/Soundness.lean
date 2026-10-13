import NecoAlgebraVerify.Soundness
import NecoCalculusVerify.Checker

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoAlgebraVerify

private theorem signChange_iff (left right : Rat) :
    NecoAlgebraVerify.signChange left right ↔ signChangeWitness left right := by
  rfl

private theorem intervalEnclosure_contains
    {outer inner : Interval} {value : Rat}
    (lowerBound : outer.lower ≤ inner.lower)
    (upperBound : inner.upper ≤ outer.upper)
    (contained : intervalContains inner value) :
    intervalContains outer value := by
  exact ⟨lowerBound.trans contained.1, contained.2.trans upperBound⟩

private theorem intervalAdd_contains
    {left right : Interval} {x y : Rat}
    (hx : intervalContains left x) (hy : intervalContains right y) :
    intervalContains (intervalAdd left right) (x + y) := by
  exact NecoAlgebraVerify.Interval.add_contains hx hy

private theorem intervalMultiply_contains
    {left right : Interval} {x y : Rat}
    (hx : intervalContains left x) (hy : intervalContains right y) :
    intervalContains (intervalMultiply left right) (x * y) := by
  exact NecoAlgebraVerify.Interval.mul_contains hx hy

private theorem intervalPower_contains
    {interval : Interval} {value : Rat} (contained : intervalContains interval value) :
    ∀ exponent : Nat, intervalContains (intervalPower interval exponent) (value ^ exponent)
  | exponent => by
    have valid : interval.Valid := contained.1.trans contained.2
    exact NecoAlgebraVerify.Interval.pow_contains exponent valid contained

private theorem intervalScale_contains
    {scalar value : Rat} {interval : Interval}
    (contained : intervalContains interval value) :
    intervalContains (intervalScale scalar interval) (scalar * value) := by
  unfold intervalContains intervalScale
  exact NecoAlgebraVerify.Interval.mul_contains
    (NecoAlgebraVerify.Interval.constant_contains scalar) contained

theorem polynomialRange_contains
    (coefficients : Polynomial) (domain : Interval) (point : Rat)
    (contained : intervalContains domain point) :
    intervalContains (polynomialRange coefficients domain)
      (NecoAlgebraVerify.eval coefficients point) := by
  have valid : domain.Valid := contained.1.trans contained.2
  exact NecoAlgebraVerify.intervalEval_sound valid contained

theorem polynomialRange_valid
    (coefficients : Polynomial) (domain : Interval)
    (_valid : intervalValid domain) :
    intervalValid (polynomialRange coefficients domain) := by
  exact NecoAlgebraVerify.intervalEval_valid coefficients domain

private theorem surfaceTermRange_valid (term : SurfaceTerm) (u v : Interval) :
    intervalValid (surfaceTermRange term u v) := by
  unfold intervalValid surfaceTermRange intervalScale intervalMultiply intervalPower
  exact NecoAlgebraVerify.Interval.mul_valid _ _

private theorem surfaceRangeFold_valid
    (terms : List SurfaceTerm) (u v total : Interval)
    (totalValid : intervalValid total) :
    intervalValid (terms.foldl
      (fun accumulator term => intervalAdd accumulator (surfaceTermRange term u v)) total) := by
  induction terms generalizing total with
  | nil => simpa using totalValid
  | cons term remaining inductionHypothesis =>
    simp only [List.foldl_cons]
    exact inductionHypothesis
      (intervalAdd total (surfaceTermRange term u v))
      (NecoAlgebraVerify.Interval.add_valid totalValid (surfaceTermRange_valid term u v))

theorem surfaceRange_valid (terms : List SurfaceTerm) (u v : Interval) :
    intervalValid (surfaceRange terms u v) := by
  unfold surfaceRange
  exact surfaceRangeFold_valid terms u v { lower := 0, upper := 0 }
    NecoAlgebraVerify.Interval.zero_valid

private theorem surfaceTermRange_contains
    (term : SurfaceTerm) (u v : Interval) (pointU pointV : Rat)
    (uContained : intervalContains u pointU)
    (vContained : intervalContains v pointV) :
    intervalContains (surfaceTermRange term u v) (monomial term pointU pointV) := by
  unfold surfaceTermRange monomial
  simpa [mul_assoc] using intervalScale_contains
    (scalar := term.coefficient)
    (interval := intervalMultiply (intervalPower u term.exponentU) (intervalPower v term.exponentV))
    (intervalMultiply_contains
      (intervalPower_contains uContained term.exponentU)
      (intervalPower_contains vContained term.exponentV))

private theorem surfaceRange_contains_aux
    (terms : List SurfaceTerm) (u v : Interval) (pointU pointV : Rat)
    (uContained : intervalContains u pointU)
    (vContained : intervalContains v pointV) :
    ∀ (total : Interval) (totalValue : Rat),
      intervalContains total totalValue →
      intervalContains
        (terms.foldl (fun accumulator term =>
          intervalAdd accumulator (surfaceTermRange term u v)) total)
        (terms.foldl (fun accumulator term =>
          accumulator + monomial term pointU pointV) totalValue) := by
  induction terms with
  | nil =>
    intro total totalValue contained
    simpa using contained
  | cons term remaining inductionHypothesis =>
    intro total totalValue contained
    simp only [List.foldl_cons]
    exact inductionHypothesis
      (intervalAdd total (surfaceTermRange term u v))
      (totalValue + monomial term pointU pointV)
      (intervalAdd_contains contained
        (surfaceTermRange_contains term u v pointU pointV uContained vContained))

theorem surfaceRange_contains
    (terms : List SurfaceTerm) (u v : Interval) (pointU pointV : Rat)
    (uContained : intervalContains u pointU)
    (vContained : intervalContains v pointV) :
    intervalContains (surfaceRange terms u v) (surfaceValue terms pointU pointV) := by
  unfold surfaceRange surfaceValue
  exact surfaceRange_contains_aux terms u v pointU pointV uContained vContained
    { lower := 0, upper := 0 } 0 (by
      unfold intervalContains NecoAlgebraVerify.Interval.Contains
      exact ⟨le_rfl, le_rfl⟩)

theorem acceptPolynomialExact_sound
    (observation : PolynomialExactObservation)
    (accepted : acceptPolynomialExact observation = true) :
    polynomialExact observation.coefficients observation.point observation.observed := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with ⟨normalized, observed⟩
  exact ⟨normalized, observed⟩

theorem acceptPolynomialEnclosure_sound
    (observation : PolynomialEnclosureObservation)
    (accepted : acceptPolynomialEnclosure observation = true) :
    polynomialEncloses observation.coefficients observation.domain observation.output := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with ⟨normalized, domainValid, outputValid, lowerBound, upperBound⟩
  refine ⟨normalized, domainValid, outputValid, ?_⟩
  intro point contained
  exact intervalEnclosure_contains lowerBound upperBound
    (NecoAlgebraVerify.intervalEval_sound domainValid contained)

theorem acceptRationalExact_sound
    (observation : RationalExactObservation)
    (accepted : acceptRationalExact observation = true) :
    rationalExact observation.numerator observation.denominator observation.point
      observation.observed := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with
    ⟨numeratorNormalized, denominatorNormalized, denominatorNonzero, observed⟩
  exact ⟨numeratorNormalized, denominatorNormalized, denominatorNonzero, observed⟩

theorem acceptRationalEnclosure_sound
    (observation : RationalEnclosureObservation)
    (accepted : acceptRationalEnclosure observation = true) :
    rationalEncloses observation.numerator observation.denominator observation.domain
      observation.output := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with
    ⟨numeratorNormalized, denominatorNormalized, domainValid, denominatorExcludesZero,
      outputValid, lowerBound, upperBound⟩
  refine ⟨numeratorNormalized, denominatorNormalized, domainValid,
    denominatorExcludesZero, outputValid, ?_⟩
  intro point contained
  have numeratorContained := NecoAlgebraVerify.intervalEval_sound
    (coefficients := observation.numerator) domainValid contained
  have denominatorContained := NecoAlgebraVerify.intervalEval_sound
    (coefficients := observation.denominator) domainValid contained
  have quotientContained := NecoAlgebraVerify.Interval.div_contains
    (NecoAlgebraVerify.intervalEval_valid observation.numerator observation.domain)
    (NecoAlgebraVerify.intervalEval_valid observation.denominator observation.domain)
    denominatorExcludesZero numeratorContained denominatorContained
  exact intervalEnclosure_contains lowerBound upperBound quotientContained

theorem acceptRootBracket_sound
    (observation : RootBracketObservation)
    (accepted : acceptRootBracket observation = true) :
    NecoAlgebraVerify.IsSignChangeBracket observation.coefficients observation.lower
      observation.upper := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with ⟨_, ordered, signChange, _⟩
  exact NecoAlgebraVerify.signChangeBracket_of_eval ordered signChange

theorem acceptBisection_sound
    (observation : BisectionObservation)
    (accepted : acceptBisection observation = true) :
    bisectionOutcome observation.coefficients observation.initialLower observation.initialUpper
      observation.finalLower observation.finalUpper observation.requestedIterations
      observation.actualIterations observation.status observation.trace := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with
    ⟨normalized, requestedPositive, requestedBounded, ordered, signChange, _, actualLength,
      aggregateReplay, traceReplay⟩
  exact ⟨normalized, requestedPositive, requestedBounded, ordered,
    (signChange_iff _ _).mp signChange, actualLength, aggregateReplay, traceReplay⟩

theorem acceptEvenTouch_sound
    (observation : EvenTouchObservation)
    (accepted : acceptEvenTouch observation = true) :
    evenTouch observation.coefficients observation.candidate observation.maximumOrder
      observation.multiplicity := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with ⟨_, firstNonzero, bounded, positiveOrder, evenOrder⟩
  exact ⟨bounded, positiveOrder, evenOrder, firstNonzero⟩

theorem acceptLocalTransition_sound
    (observation : LocalTransitionObservation)
    (accepted : acceptLocalTransition observation = true) :
    localTransition observation.coefficients observation.lower observation.candidate
      observation.upper observation.maximumOrder observation.multiplicity := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with
    ⟨_, _, lowerInterior, upperInterior, bounded, oddOrder, firstNonzero, _, _, _, _, _⟩
  exact ⟨lowerInterior, upperInterior, bounded, oddOrder, firstNonzero⟩

theorem acceptSurfaceExact_sound
    (observation : SurfaceExactObservation)
    (accepted : acceptSurfaceExact observation = true) :
    surfaceExact observation.x observation.y observation.z observation.denominator
      observation.u observation.v observation.observedX observation.observedY
      observation.observedZ := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with ⟨denominatorNonzero, observedX, observedY, observedZ⟩
  exact ⟨denominatorNonzero, observedX, observedY, observedZ⟩

theorem acceptSurfaceChecked_sound
    (observation : SurfaceCheckedObservation)
    (accepted : acceptSurfaceChecked observation = true) :
    surfaceChecked observation.x observation.y observation.z observation.denominator
      observation.u observation.v observation.output observation.inputProvenance
      observation.outputProvenance := by
  have valid : observation.Valid :=
    of_decide_eq_true (show decide observation.Valid = true from accepted)
  rcases valid with
    ⟨_, uValid, vValid, denominatorExcludesZero, outputXValid, outputYValid, outputZValid,
      provenance, xLower, xUpper, yLower, yUpper, zLower, zUpper⟩
  refine ⟨uValid, vValid, denominatorExcludesZero, outputXValid, outputYValid, outputZValid,
    provenance, ?_⟩
  intro pointU pointV uContained vContained
  have denominatorContained := surfaceRange_contains observation.denominator observation.u
    observation.v pointU pointV uContained vContained
  have denominatorValid := surfaceRange_valid observation.denominator observation.u observation.v
  have xContained := NecoAlgebraVerify.Interval.div_contains
    (surfaceRange_valid observation.x observation.u observation.v)
    denominatorValid denominatorExcludesZero
    (surfaceRange_contains observation.x observation.u observation.v pointU pointV
      uContained vContained)
    denominatorContained
  have yContained := NecoAlgebraVerify.Interval.div_contains
    (surfaceRange_valid observation.y observation.u observation.v)
    denominatorValid denominatorExcludesZero
    (surfaceRange_contains observation.y observation.u observation.v pointU pointV
      uContained vContained)
    denominatorContained
  have zContained := NecoAlgebraVerify.Interval.div_contains
    (surfaceRange_valid observation.z observation.u observation.v)
    denominatorValid denominatorExcludesZero
    (surfaceRange_contains observation.z observation.u observation.v pointU pointV
      uContained vContained)
    denominatorContained
  exact ⟨intervalEnclosure_contains xLower xUpper xContained,
    intervalEnclosure_contains yLower yUpper yContained,
    intervalEnclosure_contains zLower zUpper zContained⟩

theorem acceptSupportedObservation_sound
    (observation : SupportedObservation)
    (accepted : acceptSupportedObservation observation = true) :
    observation.Semantics := by
  cases observation with
  | polynomialExact observation =>
      exact acceptPolynomialExact_sound observation accepted
  | polynomialEnclosure observation =>
      exact acceptPolynomialEnclosure_sound observation accepted
  | polynomialExactFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | polynomialEnclosureFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | rationalExact observation =>
      exact acceptRationalExact_sound observation accepted
  | rationalEnclosure observation =>
      exact acceptRationalEnclosure_sound observation accepted
  | rationalExactFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | rationalEnclosureFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | rootBracket observation =>
      exact acceptRootBracket_sound observation accepted
  | bisection observation =>
      exact acceptBisection_sound observation accepted
  | rootBracketFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | bisectionFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | evenTouch observation =>
      exact acceptEvenTouch_sound observation accepted
  | localTransition observation =>
      exact acceptLocalTransition_sound observation accepted
  | evenTouchFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | localTransitionFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | surfaceExact observation =>
      exact acceptSurfaceExact_sound observation accepted
  | surfaceChecked observation =>
      exact acceptSurfaceChecked_sound observation accepted
  | surfaceExactFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)
  | surfaceCheckedFailure observation =>
      exact (of_decide_eq_true accepted : observation.Valid)

theorem checkCase_verified_sound (value : NecoLean.Json.Value) :
    (checkCase value).verdict = .verified →
    ∃ decoded, decodeCase value = some decoded ∧ decoded.observation.Semantics := by
  intro verified
  cases hdecode : decodeCase value with
  | none =>
      simp [checkCase, hdecode] at verified
  | some decoded =>
      refine ⟨decoded, rfl, ?_⟩
      apply acceptSupportedObservation_sound decoded.observation
      cases haccept : acceptSupportedObservation decoded.observation with
      | false =>
          simp [checkCase, hdecode, haccept] at verified
      | true => rfl

end NecoCalculusVerify
