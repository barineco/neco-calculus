import NecoAlgebraVerify.Semantics

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoAlgebraVerify

abbrev Polynomial := List Int
abbrev Interval := NecoAlgebraVerify.Interval

structure SurfaceTerm where
  coefficient : Rat
  exponentU : Nat
  exponentV : Nat
  deriving Repr, BEq, DecidableEq

structure Provenance where
  consumerId : Nat
  expressions : List Nat
  decisionKind : String
  decisionExpressions : List Nat
  owner : Nat
  precision : Nat
  sourcePublicApiIdentity : String
  sourceRevisionIdentity : String
  deriving Repr, BEq, DecidableEq

structure CoordinateIntervals where
  x : Interval
  y : Interval
  z : Interval
  deriving Repr, BEq, DecidableEq

structure BisectionTraceStep where
  midpoint : Rat
  midpointSign : String
  selection : String
  updatedLower : Rat
  updatedUpper : Rat
  deriving Repr, BEq, DecidableEq

def intervalValid (interval : Interval) : Prop :=
  interval.Valid

def intervalContains (interval : Interval) (value : Rat) : Prop :=
  interval.Contains value

def intervalAdd (left right : Interval) : Interval :=
  Interval.add left right

def intervalMultiply (left right : Interval) : Interval :=
  Interval.mul left right

def intervalDivision (numerator denominator : Interval) : Interval :=
  Interval.div numerator denominator

def intervalPower (interval : Interval) (exponent : Nat) : Interval :=
  Interval.pow interval exponent

def intervalScale (scalar : Rat) (interval : Interval) : Interval :=
  Interval.mul (Interval.constant scalar) interval

def polynomialRange (coefficients : Polynomial) (domain : Interval) : Interval :=
  NecoAlgebraVerify.intervalEval coefficients domain

def rationalValue (numerator denominator : Polynomial) (point : Rat) : Rat :=
  NecoAlgebraVerify.eval numerator point / NecoAlgebraVerify.eval denominator point

def rationalRange (numerator denominator : Polynomial) (domain : Interval) : Interval :=
  intervalDivision (polynomialRange numerator domain) (polynomialRange denominator domain)

def iterateDerivative : Polynomial → Nat → Polynomial
  | coefficients, 0 => coefficients
  | coefficients, order + 1 =>
    iterateDerivative (NecoAlgebraVerify.derivative coefficients) order

def derivativeAt (coefficients : Polynomial) (order : Nat) (point : Rat) : Rat :=
  NecoAlgebraVerify.eval (iterateDerivative coefficients order) point

def firstNonzeroOrderAux : Nat → Nat → Polynomial → Rat → Option Nat
  | 0, _, _, _ => none
  | fuel + 1, order, coefficients, point =>
    if NecoAlgebraVerify.eval coefficients point == 0 then
      firstNonzeroOrderAux fuel (order + 1) (NecoAlgebraVerify.derivative coefficients) point
    else
      some order

def firstNonzeroOrder (coefficients : Polynomial) (point : Rat) : Option Nat :=
  firstNonzeroOrderAux (coefficients.length + 1) 0 coefficients point

def monomial (term : SurfaceTerm) (u v : Rat) : Rat :=
  term.coefficient * u ^ term.exponentU * v ^ term.exponentV

def surfaceValue (terms : List SurfaceTerm) (u v : Rat) : Rat :=
  terms.foldl (fun total term => total + monomial term u v) 0

def surfaceTermRange (term : SurfaceTerm) (u v : Interval) : Interval :=
  intervalScale term.coefficient (intervalMultiply (intervalPower u term.exponentU)
    (intervalPower v term.exponentV))

def surfaceRange (terms : List SurfaceTerm) (u v : Interval) : Interval :=
  terms.foldl (fun total term => intervalAdd total (surfaceTermRange term u v))
    { lower := 0, upper := 0 }

def bisectionSignText (value : Rat) : String :=
  if value < 0 then "negative" else if value == 0 then "zero" else "positive"

def bisectionReplay : Nat → Polynomial → Rat → Rat → Option (Rat × Rat × String)
  | 0, _, lower, upper => some (lower, upper, "refined")
  | fuel + 1, coefficients, lower, upper =>
    let midpoint := (lower + upper) / 2
    if NecoAlgebraVerify.eval coefficients midpoint == 0 then
      some (midpoint, midpoint, "exact-dyadic-root")
    else if NecoAlgebraVerify.signChange
        (NecoAlgebraVerify.eval coefficients lower)
        (NecoAlgebraVerify.eval coefficients midpoint) then
      bisectionReplay fuel coefficients lower midpoint
    else if NecoAlgebraVerify.signChange
        (NecoAlgebraVerify.eval coefficients midpoint)
        (NecoAlgebraVerify.eval coefficients upper) then
      bisectionReplay fuel coefficients midpoint upper
    else
      none

def bisectionTraceReplay : Nat → Polynomial → Rat → Rat → List BisectionTraceStep →
    Option (Rat × Rat × Nat × String)
  | 0, _, lower, upper, [] => some (lower, upper, 0, "refined")
  | 0, _, _, _, _ :: _ => none
  | fuel + 1, coefficients, lower, upper, step :: rest =>
    let midpoint := (lower + upper) / 2
    let midpointSign := bisectionSignText (NecoAlgebraVerify.eval coefficients midpoint)
    if lower >= upper || step.midpoint != midpoint || step.midpointSign != midpointSign then
      none
    else if midpointSign == "zero" then
      if step.selection != "exact-root" || step.updatedLower != midpoint ||
          step.updatedUpper != midpoint || !rest.isEmpty then
        none
      else
        some (midpoint, midpoint, 1, "exact-dyadic-root")
    else if NecoAlgebraVerify.signChange
        (NecoAlgebraVerify.eval coefficients lower)
        (NecoAlgebraVerify.eval coefficients midpoint) then
      if step.selection != "lower-half" || step.updatedLower != lower ||
          step.updatedUpper != midpoint then
        none
      else
        match bisectionTraceReplay fuel coefficients lower midpoint rest with
        | some (finalLower, finalUpper, iterations, status) =>
          some (finalLower, finalUpper, iterations + 1, status)
        | none => none
    else if NecoAlgebraVerify.signChange
        (NecoAlgebraVerify.eval coefficients midpoint)
        (NecoAlgebraVerify.eval coefficients upper) then
      if step.selection != "upper-half" || step.updatedLower != midpoint ||
          step.updatedUpper != upper then
        none
      else
        match bisectionTraceReplay fuel coefficients midpoint upper rest with
        | some (finalLower, finalUpper, iterations, status) =>
          some (finalLower, finalUpper, iterations + 1, status)
        | none => none
    else
      none
  | _, _, _, _, [] => none

def normalizedPolynomial (coefficients : Polynomial) : Prop :=
  coefficients ≠ [] ∧
    (coefficients.length = 1 ∨ coefficients.getLast? ≠ some 0)

instance (coefficients : Polynomial) : Decidable (normalizedPolynomial coefficients) := by
  unfold normalizedPolynomial
  infer_instance

def polynomialExact (coefficients : Polynomial) (point observed : Rat) : Prop :=
  normalizedPolynomial coefficients ∧
  observed = NecoAlgebraVerify.eval coefficients point

def polynomialEncloses (coefficients : Polynomial) (domain output : Interval) : Prop :=
  normalizedPolynomial coefficients ∧
  intervalValid domain ∧
  intervalValid output ∧
  ∀ point, intervalContains domain point →
    intervalContains output (NecoAlgebraVerify.eval coefficients point)

def rationalExact (numerator denominator : Polynomial) (point observed : Rat) : Prop :=
  normalizedPolynomial numerator ∧
  normalizedPolynomial denominator ∧
  NecoAlgebraVerify.eval denominator point ≠ 0 ∧
  observed = rationalValue numerator denominator point

def rationalEncloses (numerator denominator : Polynomial) (domain output : Interval) : Prop :=
  normalizedPolynomial numerator ∧
  normalizedPolynomial denominator ∧
  intervalValid domain ∧
  ¬ intervalContains (polynomialRange denominator domain) 0 ∧
  intervalValid output ∧
  ∀ point, intervalContains domain point →
    intervalContains output (rationalValue numerator denominator point)

def exactUndefinedDomain (denominator : Polynomial) (point : Rat) : Prop :=
  NecoAlgebraVerify.eval denominator point = 0

def intervalDomainContainsZero (denominator : Polynomial) (domain : Interval) : Prop :=
  intervalContains (polynomialRange denominator domain) 0

instance (denominator : Polynomial) (point : Rat) :
    Decidable (exactUndefinedDomain denominator point) := by
  unfold exactUndefinedDomain
  infer_instance

instance (denominator : Polynomial) (domain : Interval) :
    Decidable (intervalDomainContainsZero denominator domain) := by
  unfold intervalDomainContainsZero intervalContains
  infer_instance

def signChangeWitness (left right : Rat) : Prop :=
  (left < 0 ∧ 0 < right) ∨ (right < 0 ∧ 0 < left)

def bisectionOutcome
    (coefficients : Polynomial) (initialLower initialUpper finalLower finalUpper : Rat)
    (requestedIterations actualIterations : Nat) (status : String)
    (trace : List BisectionTraceStep) : Prop :=
  normalizedPolynomial coefficients ∧
  0 < requestedIterations ∧
  requestedIterations ≤ 19 ∧
  initialLower < initialUpper ∧
  signChangeWitness (NecoAlgebraVerify.eval coefficients initialLower)
    (NecoAlgebraVerify.eval coefficients initialUpper) ∧
  actualIterations = trace.length ∧
  bisectionReplay requestedIterations coefficients initialLower initialUpper =
    some (finalLower, finalUpper, status) ∧
  bisectionTraceReplay requestedIterations coefficients initialLower initialUpper trace =
    some (finalLower, finalUpper, actualIterations, status)

def evenTouch
    (coefficients : Polynomial) (candidate : Rat) (maximumOrder multiplicity : Nat) : Prop :=
  multiplicity ≤ maximumOrder ∧
  2 ≤ multiplicity ∧
  multiplicity % 2 = 0 ∧
  firstNonzeroOrder coefficients candidate = some multiplicity

def localTransition
    (coefficients : Polynomial) (lower candidate upper : Rat)
    (maximumOrder multiplicity : Nat) : Prop :=
  lower < candidate ∧ candidate < upper ∧
  multiplicity ≤ maximumOrder ∧
  multiplicity % 2 = 1 ∧
  firstNonzeroOrder coefficients candidate = some multiplicity

def surfaceExact
    (x y z denominator : List SurfaceTerm) (u v observedX observedY observedZ : Rat) : Prop :=
  surfaceValue denominator u v ≠ 0 ∧
  observedX = surfaceValue x u v / surfaceValue denominator u v ∧
  observedY = surfaceValue y u v / surfaceValue denominator u v ∧
  observedZ = surfaceValue z u v / surfaceValue denominator u v

def surfaceChecked
    (x y z denominator : List SurfaceTerm) (u v : Interval)
    (output : CoordinateIntervals) (inputProvenance outputProvenance : Provenance) : Prop :=
  intervalValid u ∧
  intervalValid v ∧
  ¬ intervalContains (surfaceRange denominator u v) 0 ∧
  intervalValid output.x ∧
  intervalValid output.y ∧
  intervalValid output.z ∧
  outputProvenance = inputProvenance ∧
  ∀ pointU pointV,
    intervalContains u pointU →
    intervalContains v pointV →
    intervalContains output.x
      (surfaceValue x pointU pointV / surfaceValue denominator pointU pointV) ∧
    intervalContains output.y
      (surfaceValue y pointU pointV / surfaceValue denominator pointU pointV) ∧
    intervalContains output.z
      (surfaceValue z pointU pointV / surfaceValue denominator pointU pointV)

end NecoCalculusVerify
