import NecoCalculusVerify.Protocol
import NecoCalculusVerify.Semantics

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

inductive ObservationPayload where
  | success (value : Value)
  | failure (name : String)
  deriving Repr, BEq

structure PolynomialExactObservation where
  coefficients : Polynomial
  point : Rat
  observed : Rat
  deriving Repr, BEq, DecidableEq

structure PolynomialEnclosureObservation where
  coefficients : Polynomial
  domain : Interval
  precisionBits : Nat
  output : Interval
  deriving Repr, BEq, DecidableEq

structure PolynomialExactFailureObservation where
  coefficients : Polynomial
  point : Rat
  failure : String
  deriving Repr, BEq, DecidableEq

structure PolynomialEnclosureFailureObservation where
  coefficients : Polynomial
  domain : Interval
  precisionBits : Nat
  failure : String
  deriving Repr, BEq, DecidableEq

structure RationalExactObservation where
  numerator : Polynomial
  denominator : Polynomial
  point : Rat
  observed : Rat
  deriving Repr, BEq, DecidableEq

structure RationalEnclosureObservation where
  numerator : Polynomial
  denominator : Polynomial
  domain : Interval
  precisionBits : Nat
  output : Interval
  deriving Repr, BEq, DecidableEq

structure RationalExactFailureObservation where
  numerator : Polynomial
  denominator : Polynomial
  point : Rat
  failure : String
  deriving Repr, BEq, DecidableEq

structure RationalEnclosureFailureObservation where
  numerator : Polynomial
  denominator : Polynomial
  domain : Interval
  precisionBits : Nat
  failure : String
  deriving Repr, BEq, DecidableEq

structure RootBracketObservation where
  coefficients : Polynomial
  lower : Rat
  upper : Rat
  evidence : String
  deriving Repr, BEq, DecidableEq

structure BisectionObservation where
  coefficients : Polynomial
  requestedIterations : Nat
  initialLower : Rat
  initialUpper : Rat
  finalLower : Rat
  finalUpper : Rat
  actualIterations : Nat
  method : String
  status : String
  trace : List BisectionTraceStep
  deriving Repr, BEq, DecidableEq

structure RootBracketFailureObservation where
  coefficients : Polynomial
  lower : Rat
  upper : Rat
  failure : String
  deriving Repr, BEq, DecidableEq

structure BisectionFailureObservation where
  coefficients : Polynomial
  requestedIterations : Nat
  initialLower : Rat
  initialUpper : Rat
  failure : String
  deriving Repr, BEq, DecidableEq

structure EvenTouchObservation where
  coefficients : Polynomial
  candidate : Rat
  maximumOrder : Nat
  multiplicity : Nat
  deriving Repr, BEq, DecidableEq

structure LocalTransitionObservation where
  coefficients : Polynomial
  lower : Rat
  candidate : Rat
  upper : Rat
  maximumOrder : Nat
  precisionBits : Nat
  locusCount : Nat
  lociLength : Nat
  kind : String
  multiplicity : Nat
  pointIndex : Nat
  sign : String
  deriving Repr, BEq, DecidableEq

structure EvenTouchFailureObservation where
  coefficients : Polynomial
  candidate : Rat
  maximumOrder : Nat
  failure : String
  deriving Repr, BEq, DecidableEq

structure LocalTransitionFailureObservation where
  coefficients : Polynomial
  lower : Rat
  candidate : Rat
  upper : Rat
  maximumOrder : Nat
  precisionBits : Nat
  failure : String
  deriving Repr, BEq, DecidableEq

structure SurfaceExactObservation where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Rat
  v : Rat
  observedX : Rat
  observedY : Rat
  observedZ : Rat
  deriving Repr, BEq, DecidableEq

structure SurfaceCheckedObservation where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Interval
  v : Interval
  precisionBits : Nat
  output : CoordinateIntervals
  inputProvenance : Provenance
  outputProvenance : Provenance
  deriving Repr, BEq, DecidableEq

structure SurfaceExactFailureObservation where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Rat
  v : Rat
  failure : String
  deriving Repr, BEq, DecidableEq

structure SurfaceCheckedFailureObservation where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Interval
  v : Interval
  precisionBits : Nat
  inputProvenance : Provenance
  failure : String
  deriving Repr, BEq, DecidableEq

def PolynomialExactFailureObservation.Semantics
    (observation : PolynomialExactFailureObservation) : Prop :=
  observation.failure = "empty-coefficients" ∧ observation.coefficients = []

def PolynomialEnclosureFailureObservation.Semantics
    (observation : PolynomialEnclosureFailureObservation) : Prop :=
  observation.failure = "precision-limit-exceeded" ∧
    normalizedPolynomial observation.coefficients ∧ observation.domain.Valid ∧
    256 < observation.precisionBits

def RationalExactFailureObservation.Semantics
    (observation : RationalExactFailureObservation) : Prop :=
  observation.failure = "undefined-domain" ∧
    normalizedPolynomial observation.numerator ∧
    normalizedPolynomial observation.denominator ∧
    exactUndefinedDomain observation.denominator observation.point

def RationalEnclosureFailureObservation.Semantics
    (observation : RationalEnclosureFailureObservation) : Prop :=
  observation.failure = "domain-contains-zero" ∧
    normalizedPolynomial observation.numerator ∧
    normalizedPolynomial observation.denominator ∧ observation.domain.Valid ∧
    intervalDomainContainsZero observation.denominator observation.domain

def RootBracketFailureObservation.Semantics
    (observation : RootBracketFailureObservation) : Prop :=
  observation.failure = "no-sign-change" ∧
    normalizedPolynomial observation.coefficients ∧ observation.lower < observation.upper ∧
    ¬ signChange (eval observation.coefficients observation.lower)
      (eval observation.coefficients observation.upper)

def BisectionFailureObservation.Semantics
    (observation : BisectionFailureObservation) : Prop :=
  observation.failure = "refinement-iterations-exceeded" ∧
    normalizedPolynomial observation.coefficients ∧ 19 < observation.requestedIterations ∧
    observation.initialLower < observation.initialUpper ∧
    signChange (eval observation.coefficients observation.initialLower)
      (eval observation.coefficients observation.initialUpper)

def EvenTouchFailureObservation.Semantics
    (observation : EvenTouchFailureObservation) : Prop :=
  observation.failure = "no-even-multiplicity-touch" ∧
    normalizedPolynomial observation.coefficients ∧
    match firstNonzeroOrder observation.coefficients observation.candidate with
    | some order => order ≤ observation.maximumOrder ∧ (order < 2 ∨ order % 2 = 1)
    | none => True

def LocalTransitionFailureObservation.Semantics
    (observation : LocalTransitionFailureObservation) : Prop :=
  observation.failure = "point-outside-enclosure" ∧
    normalizedPolynomial observation.coefficients ∧ observation.lower ≤ observation.upper ∧
    (observation.candidate < observation.lower ∨ observation.upper < observation.candidate)

def SurfaceExactFailureObservation.Semantics
    (observation : SurfaceExactFailureObservation) : Prop :=
  observation.failure = "undefined-domain" ∧
    surfaceValue observation.denominator observation.u observation.v = 0

def SurfaceCheckedFailureObservation.Semantics
    (observation : SurfaceCheckedFailureObservation) : Prop :=
  observation.failure = "zero-denominator-polynomial" ∧ 0 < observation.precisionBits ∧
    observation.u.Valid ∧ observation.v.Valid ∧ observation.denominator = []

inductive SupportedObservation where
  | polynomialExact (observation : PolynomialExactObservation)
  | polynomialEnclosure (observation : PolynomialEnclosureObservation)
  | polynomialExactFailure (observation : PolynomialExactFailureObservation)
  | polynomialEnclosureFailure (observation : PolynomialEnclosureFailureObservation)
  | rationalExact (observation : RationalExactObservation)
  | rationalEnclosure (observation : RationalEnclosureObservation)
  | rationalExactFailure (observation : RationalExactFailureObservation)
  | rationalEnclosureFailure (observation : RationalEnclosureFailureObservation)
  | rootBracket (observation : RootBracketObservation)
  | bisection (observation : BisectionObservation)
  | rootBracketFailure (observation : RootBracketFailureObservation)
  | bisectionFailure (observation : BisectionFailureObservation)
  | evenTouch (observation : EvenTouchObservation)
  | localTransition (observation : LocalTransitionObservation)
  | evenTouchFailure (observation : EvenTouchFailureObservation)
  | localTransitionFailure (observation : LocalTransitionFailureObservation)
  | surfaceExact (observation : SurfaceExactObservation)
  | surfaceChecked (observation : SurfaceCheckedObservation)
  | surfaceExactFailure (observation : SurfaceExactFailureObservation)
  | surfaceCheckedFailure (observation : SurfaceCheckedFailureObservation)
  deriving Repr, BEq, DecidableEq

def SupportedObservation.Semantics : SupportedObservation → Prop
  | .polynomialExact observation =>
      NecoCalculusVerify.polynomialExact observation.coefficients observation.point
        observation.observed
  | .polynomialEnclosure observation =>
      polynomialEncloses observation.coefficients observation.domain observation.output
  | .polynomialExactFailure observation => observation.Semantics
  | .polynomialEnclosureFailure observation => observation.Semantics
  | .rationalExact observation =>
      NecoCalculusVerify.rationalExact observation.numerator observation.denominator
        observation.point observation.observed
  | .rationalEnclosure observation =>
      rationalEncloses observation.numerator observation.denominator observation.domain
        observation.output
  | .rationalExactFailure observation => observation.Semantics
  | .rationalEnclosureFailure observation => observation.Semantics
  | .rootBracket observation =>
      IsSignChangeBracket observation.coefficients observation.lower observation.upper
  | .bisection observation =>
      bisectionOutcome observation.coefficients observation.initialLower observation.initialUpper
        observation.finalLower observation.finalUpper observation.requestedIterations
        observation.actualIterations observation.status observation.trace
  | .rootBracketFailure observation => observation.Semantics
  | .bisectionFailure observation => observation.Semantics
  | .evenTouch observation =>
      NecoCalculusVerify.evenTouch observation.coefficients observation.candidate
        observation.maximumOrder observation.multiplicity
  | .localTransition observation =>
      NecoCalculusVerify.localTransition observation.coefficients observation.lower
        observation.candidate observation.upper observation.maximumOrder observation.multiplicity
  | .evenTouchFailure observation => observation.Semantics
  | .localTransitionFailure observation => observation.Semantics
  | .surfaceExact observation =>
      NecoCalculusVerify.surfaceExact observation.x observation.y observation.z
        observation.denominator observation.u observation.v observation.observedX
        observation.observedY observation.observedZ
  | .surfaceChecked observation =>
      NecoCalculusVerify.surfaceChecked observation.x observation.y observation.z
        observation.denominator observation.u observation.v observation.output
        observation.inputProvenance observation.outputProvenance
  | .surfaceExactFailure observation => observation.Semantics
  | .surfaceCheckedFailure observation => observation.Semantics

structure DecodedCase where
  caseId : String
  observation : SupportedObservation
  deriving Repr, BEq, DecidableEq

def decodeObservationPayload (value : Value) : Option ObservationPayload :=
  match jsonObjectWithExactFields? value ["kind", "value"],
      jsonObjectWithExactFields? value ["failure", "kind"] with
  | some fields, _ => do
      if (← jsonStringField? fields "kind") != "success" then none
      pure (.success (← jsonField? fields "value"))
  | _, some fields => do
      if (← jsonStringField? fields "kind") != "failure" then none
      pure (.failure (← jsonStringField? fields "failure"))
  | _, _ => none

end NecoCalculusVerify
