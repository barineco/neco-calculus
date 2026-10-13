import NecoCalculusVerify

open NecoCalculusVerify

private def verified : CheckResult → Bool
  | .batchInvalid _ => false
  | .cases _ items => !items.isEmpty && items.all (fun item => item.verdict == .verified)

private def expectVerdict (name : String) (expected : Verdict) : CheckResult → IO Unit
  | .cases _ (item :: _) =>
      if item.verdict == expected then pure ()
      else throw (IO.userError s!"{name}: unexpected verdict")
  | _ => throw (IO.userError s!"{name}: expected a case verdict")

private def expectBatchInvalid (name : String) : CheckResult → IO Unit
  | .batchInvalid _ => pure ()
  | .cases _ _ => throw (IO.userError s!"{name}: expected batch-invalid result")

private def resultFromPath (path : String) : IO CheckResult := do
  let bytes ← IO.FS.readBinFile path
  pure (checkBytes bytes)

private theorem linearExactSoundnessWitness
    (observation : PolynomialExactObservation)
    (accepted : acceptPolynomialExact observation = true) :
    polynomialExact observation.coefficients observation.point observation.observed :=
  acceptPolynomialExact_sound observation accepted

private theorem linearEnclosureSoundnessWitness
    (observation : PolynomialEnclosureObservation)
    (accepted : acceptPolynomialEnclosure observation = true) :
    polynomialEncloses observation.coefficients observation.domain observation.output :=
  acceptPolynomialEnclosure_sound observation accepted

private theorem rationalExactSoundnessWitness
    (observation : RationalExactObservation)
    (accepted : acceptRationalExact observation = true) :
    rationalExact observation.numerator observation.denominator observation.point
      observation.observed :=
  acceptRationalExact_sound observation accepted

private theorem rationalEnclosureSoundnessWitness
    (observation : RationalEnclosureObservation)
    (accepted : acceptRationalEnclosure observation = true) :
    rationalEncloses observation.numerator observation.denominator observation.domain
      observation.output :=
  acceptRationalEnclosure_sound observation accepted

private theorem bracketSoundnessWitness
    (observation : RootBracketObservation)
    (accepted : acceptRootBracket observation = true) :
    NecoAlgebraVerify.IsSignChangeBracket observation.coefficients observation.lower
      observation.upper :=
  acceptRootBracket_sound observation accepted

private theorem bisectionSoundnessWitness
    (observation : BisectionObservation)
    (accepted : acceptBisection observation = true) :
    bisectionOutcome observation.coefficients observation.initialLower observation.initialUpper
      observation.finalLower observation.finalUpper observation.requestedIterations
      observation.actualIterations observation.status observation.trace :=
  acceptBisection_sound observation accepted

private theorem evenTouchSoundnessWitness
    (observation : EvenTouchObservation)
    (accepted : acceptEvenTouch observation = true) :
    evenTouch observation.coefficients observation.candidate observation.maximumOrder
      observation.multiplicity :=
  acceptEvenTouch_sound observation accepted

private theorem localTransitionSoundnessWitness
    (observation : LocalTransitionObservation)
    (accepted : acceptLocalTransition observation = true) :
    localTransition observation.coefficients observation.lower observation.candidate
      observation.upper observation.maximumOrder observation.multiplicity :=
  acceptLocalTransition_sound observation accepted

private theorem surfaceExactSoundnessWitness
    (observation : SurfaceExactObservation)
    (accepted : acceptSurfaceExact observation = true) :
    surfaceExact observation.x observation.y observation.z observation.denominator
      observation.u observation.v observation.observedX observation.observedY
      observation.observedZ :=
  acceptSurfaceExact_sound observation accepted

private theorem surfaceCheckedSoundnessWitness
    (observation : SurfaceCheckedObservation)
    (accepted : acceptSurfaceChecked observation = true) :
    surfaceChecked observation.x observation.y observation.z observation.denominator
      observation.u observation.v observation.output observation.inputProvenance
      observation.outputProvenance :=
  acceptSurfaceChecked_sound observation accepted

private theorem supportedObservationSoundnessWitness
    (observation : SupportedObservation)
    (accepted : acceptSupportedObservation observation = true) : observation.Semantics :=
  acceptSupportedObservation_sound observation accepted

private theorem jsonCaseTypedSoundnessWitness
    (value : NecoLean.Json.Value)
    (verified : (checkCase value).verdict = .verified) :
    ∃ decoded, decodeCase value = some decoded ∧ decoded.observation.Semantics :=
  checkCase_verified_sound value verified

private def testKnown : IO Unit := do
  let bytes ← IO.FS.readBinFile "../vectors/known/families.json"
  if verified (checkBytes bytes) then IO.println "known tests passed" else throw (IO.userError "known batch")

private def testAdversarial : IO Unit := do
  expectVerdict "tampered polynomial" .refuted (checkBytes "{\"cases\":[{\"caseId\":\"bad\",\"input\":{\"coefficients\":[\"0\",\"0\",\"1\"],\"point\":\"2\"},\"observation\":{\"kind\":\"success\",\"value\":{\"value\":\"5\"}},\"operation\":\"polynomial-exact\"}],\"schema\":\"neco.calculus.observation-batch.v1\"}".toUTF8)
  expectVerdict "incorrect failure tag" .invalid (checkBytes "{\"cases\":[{\"caseId\":\"failure\",\"input\":{\"denominator\":[\"0\",\"1\"],\"numerator\":[\"4\"],\"point\":\"0\"},\"observation\":{\"failure\":\"no-sign-change\",\"kind\":\"failure\"},\"operation\":\"rational-exact\"}],\"schema\":\"neco.calculus.observation-batch.v1\"}".toUTF8)
  expectVerdict "unknown operation" .invalid (checkBytes "{\"cases\":[{\"caseId\":\"unknown\",\"input\":{},\"observation\":{\"kind\":\"success\",\"value\":{}},\"operation\":\"unknown\"}],\"schema\":\"neco.calculus.observation-batch.v1\"}".toUTF8)
  expectBatchInvalid "noncanonical" (checkBytes "{ \"cases\":[],\"schema\":\"neco.calculus.observation-batch.v1\"}".toUTF8)
  IO.println "adversarial tests passed"

private def testClassification : IO Unit := do
  expectVerdict "reversed rational interval" .refuted
    (← resultFromPath "../vectors/tampered/rational-interval-reversed.json")
  expectVerdict "zero precision surface" .refuted
    (← resultFromPath "../vectors/tampered/surface-checked-zero-precision.json")
  expectVerdict "rational precision range" .invalid
    (← resultFromPath "../vectors/invalid/rational-interval-precision-range.json")
  expectVerdict "dyadic exponent range" .invalid
    (← resultFromPath "../vectors/invalid/dyadic-exponent-range.json")
  expectVerdict "unreduced surface coefficient" .invalid
    (← resultFromPath "../vectors/invalid/unreduced-surface-coefficient.json")
  expectBatchInvalid "duplicate caseId"
    (← resultFromPath "../vectors/invalid/duplicate-case-id.json")
  IO.println "classification tests passed"

private def testBisectionTrace : IO Unit := do
  expectVerdict "tampered bisection trace" .refuted
    (← resultFromPath "../vectors/tampered/bisection.json")
  IO.println "bisection trace tests passed"

private def testSoundness : IO Unit := IO.println "soundness theorems compiled"

private def checkPath (path : String) : IO Unit := do
  let bytes ← if path == "-" then
    (← IO.getStdin).readBinToEnd
  else IO.FS.readBinFile path
  (← IO.getStdout).write (resultBytes (checkBytes bytes))

def main (arguments : List String) : IO Unit :=
  let command := if arguments.head? == some "--" then arguments.drop 1 else arguments
  match command with
  | ["known"] => testKnown
  | ["classification"] => testClassification
  | ["bisection-trace"] => testBisectionTrace
  | ["soundness"] => testSoundness
  | ["all"] => do
      testKnown
      testAdversarial
      testClassification
      testBisectionTrace
      testSoundness
  | ["check", path] => checkPath path
  | _ => throw (IO.userError
      "usage: neco-calculus-checker <known|classification|bisection-trace|soundness|all|check PATH>")
