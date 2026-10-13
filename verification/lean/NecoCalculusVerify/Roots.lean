import NecoCalculusVerify.Observation

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

structure RootBracketInput where
  coefficients : Polynomial
  lower : Rat
  upper : Rat

structure BisectionInput where
  coefficients : Polynomial
  requestedIterations : Nat
  initialLower : Rat
  initialUpper : Rat

private def decodeBracketInput (value : Value) : Option RootBracketInput := do
  let fields ← jsonObjectWithExactFields? value ["coefficients", "lower", "upper"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let lower ← decodeExactIntegerRat (← jsonField? fields "lower")
  let upper ← decodeExactIntegerRat (← jsonField? fields "upper")
  pure { coefficients, lower, upper }

private def decodeBisectionInput (value : Value) : Option BisectionInput := do
  let fields ← jsonObjectWithExactFields? value ["coefficients", "iterations", "lower", "upper"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let requestedIterations ← jsonNatField? fields "iterations"
  let initialLower ← decodeExactIntegerRat (← jsonField? fields "lower")
  let initialUpper ← decodeExactIntegerRat (← jsonField? fields "upper")
  pure { coefficients, requestedIterations, initialLower, initialUpper }

def RootBracketObservation.Valid (observation : RootBracketObservation) : Prop :=
  normalizedPolynomial observation.coefficients ∧
    observation.lower < observation.upper ∧
    signChange (eval observation.coefficients observation.lower)
      (eval observation.coefficients observation.upper) ∧
    observation.evidence = "sign-change"

instance rootBracketObservationDecidableValid
    (observation : RootBracketObservation) : Decidable observation.Valid := by
  unfold RootBracketObservation.Valid
  infer_instance

def BisectionObservation.Valid (observation : BisectionObservation) : Prop :=
  normalizedPolynomial observation.coefficients ∧
    0 < observation.requestedIterations ∧
    observation.requestedIterations ≤ 19 ∧
    observation.initialLower < observation.initialUpper ∧
    signChange (eval observation.coefficients observation.initialLower)
      (eval observation.coefficients observation.initialUpper) ∧
    observation.method = "bisection" ∧
    observation.actualIterations = observation.trace.length ∧
    bisectionReplay observation.requestedIterations observation.coefficients
        observation.initialLower observation.initialUpper =
      some (observation.finalLower, observation.finalUpper, observation.status) ∧
    bisectionTraceReplay observation.requestedIterations observation.coefficients
        observation.initialLower observation.initialUpper observation.trace =
      some (observation.finalLower, observation.finalUpper,
        observation.actualIterations, observation.status)

instance bisectionObservationDecidableValid
    (observation : BisectionObservation) : Decidable observation.Valid := by
  unfold BisectionObservation.Valid
  infer_instance

def RootBracketFailureObservation.Valid
    (observation : RootBracketFailureObservation) : Prop := observation.Semantics

instance rootBracketFailureObservationDecidableValid
    (observation : RootBracketFailureObservation) : Decidable observation.Valid := by
  unfold RootBracketFailureObservation.Valid RootBracketFailureObservation.Semantics
  infer_instance

def BisectionFailureObservation.Valid
    (observation : BisectionFailureObservation) : Prop := observation.Semantics

instance bisectionFailureObservationDecidableValid
    (observation : BisectionFailureObservation) : Decidable observation.Valid := by
  unfold BisectionFailureObservation.Valid BisectionFailureObservation.Semantics
  infer_instance

def acceptRootBracket (observation : RootBracketObservation) : Bool :=
  decide observation.Valid

def acceptBisection (observation : BisectionObservation) : Bool :=
  decide observation.Valid

def acceptRootBracketFailure (observation : RootBracketFailureObservation) : Bool :=
  decide observation.Valid

def acceptBisectionFailure (observation : BisectionFailureObservation) : Bool :=
  decide observation.Valid

private def decodeTraceStep (value : Value) : Option BisectionTraceStep := do
  let fields ← jsonObjectWithExactFields? value
    ["midpoint", "midpointSign", "selection", "updatedLower", "updatedUpper"]
  let midpoint ← decodeDyadicRatSchema (← jsonField? fields "midpoint")
  let midpointSign ← decodeStringIn fields "midpointSign" ["negative", "zero", "positive"]
  let selection ← decodeStringIn fields "selection" ["lower-half", "upper-half", "exact-root"]
  let updatedLower ← decodeDyadicRatSchema (← jsonField? fields "updatedLower")
  let updatedUpper ← decodeDyadicRatSchema (← jsonField? fields "updatedUpper")
  pure { midpoint, midpointSign, selection, updatedLower, updatedUpper }

private def decodeTrace (value : Value) : Option (List BisectionTraceStep) :=
  match value with
  | .array values =>
      if values.isEmpty || values.length > 19 then none else values.mapM decodeTraceStep
  | _ => none

private structure BisectionOutput where
  finalLower : Rat
  finalUpper : Rat
  actualIterations : Nat
  method : String
  status : String
  trace : List BisectionTraceStep

private def decodeBisectionOutput (value : Value) : Option BisectionOutput := do
  let fields ← jsonObjectWithExactFields? value
    ["finalLower", "finalUpper", "iterations", "method", "status", "trace"]
  let method ← decodeStringIn fields "method" ["bisection"]
  let status ← decodeStringIn fields "status" ["exact-dyadic-root", "refined"]
  let actualIterations ← decodeBisectionSuccessIterations fields "iterations"
  let finalLower ← decodeDyadicRatSchema (← jsonField? fields "finalLower")
  let finalUpper ← decodeDyadicRatSchema (← jsonField? fields "finalUpper")
  let trace ← decodeTrace (← jsonField? fields "trace")
  pure { finalLower, finalUpper, actualIterations, method, status, trace }

def decodeRootBracket
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeBracketInput input
  match payload with
  | .success value => do
      let fields ← jsonObjectWithExactFields? value ["evidence"]
      let evidence ← decodeStringIn fields "evidence" ["sign-change"]
      pure (.rootBracket {
        coefficients := decodedInput.coefficients
        lower := decodedInput.lower
        upper := decodedInput.upper
        evidence
      })
  | .failure name => do
      if name != "no-sign-change" then none
      pure (.rootBracketFailure {
        coefficients := decodedInput.coefficients
        lower := decodedInput.lower
        upper := decodedInput.upper
        failure := name
      })

def decodeBisection
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeBisectionInput input
  match payload with
  | .success value => do
      if decodedInput.requestedIterations < 1 || 19 < decodedInput.requestedIterations then none
      let output ← decodeBisectionOutput value
      pure (.bisection {
        coefficients := decodedInput.coefficients
        requestedIterations := decodedInput.requestedIterations
        initialLower := decodedInput.initialLower
        initialUpper := decodedInput.initialUpper
        finalLower := output.finalLower
        finalUpper := output.finalUpper
        actualIterations := output.actualIterations
        method := output.method
        status := output.status
        trace := output.trace
      })
  | .failure name => do
      if decodedInput.requestedIterations < 20 || name != "refinement-iterations-exceeded" then none
      pure (.bisectionFailure {
        coefficients := decodedInput.coefficients
        requestedIterations := decodedInput.requestedIterations
        initialLower := decodedInput.initialLower
        initialUpper := decodedInput.initialUpper
        failure := name
      })

end NecoCalculusVerify
