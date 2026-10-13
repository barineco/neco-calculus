import NecoLean.Digest
import NecoCalculusVerify.SupportedObservation

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

private def decodeSupportedObservation
    (operation : String) (input : Value) (payload : ObservationPayload) :
    Option SupportedObservation :=
  match operation with
  | "polynomial-exact" => decodePolynomialExact input payload
  | "polynomial-enclosure" => decodePolynomialEnclosure input payload
  | "rational-exact" => decodeRationalExact input payload
  | "rational-interval" => decodeRationalEnclosure input payload
  | "root-bracket" => decodeRootBracket input payload
  | "bisection" => decodeBisection input payload
  | "even-touch" => decodeEvenTouch input payload
  | "local-region-transition" => decodeLocalTransition input payload
  | "surface-exact" => decodeSurfaceExact input payload
  | "surface-checked" => decodeSurfaceChecked input payload
  | _ => none

def decodeCase (value : Value) : Option DecodedCase := do
  let fields ← jsonObjectWithExactFields? value ["caseId", "input", "observation", "operation"]
  let caseId ← jsonStringField? fields "caseId"
  if caseId.isEmpty then none
  let operation ← jsonStringField? fields "operation"
  let input ← jsonField? fields "input"
  let payload ← decodeObservationPayload (← jsonField? fields "observation")
  let observation ← decodeSupportedObservation operation input payload
  pure { caseId, observation }

private def encodedCaseId (value : Value) : String :=
  match jsonObjectWithExactFields? value ["caseId", "input", "observation", "operation"] with
  | some fields => (jsonStringField? fields "caseId").getD ""
  | none => ""

def checkCase (value : Value) : CaseResult :=
  match decodeCase value with
  | none => { caseId := encodedCaseId value, verdict := .invalid }
  | some decoded =>
      match acceptSupportedObservation decoded.observation with
      | true => { caseId := decoded.caseId, verdict := .verified }
      | false => { caseId := decoded.caseId, verdict := .refuted }

private def jsonWhitespace (character : Char) : Bool :=
  character == ' ' || character == '\n' || character == '\r' || character == '\t'

private partial def scanBatchCases (remaining : List Char) (current : List Char)
    (cases : List (List Char)) (depth : Nat) (inString escaped : Bool) :
    Option (List (List Char)) :=
  match remaining with
  | [] =>
      if depth == 0 && !inString && !current.isEmpty then
        some (cases.reverse ++ [current.reverse])
      else none
  | character :: rest =>
      if inString then
        if escaped then scanBatchCases rest (character :: current) cases depth true false
        else if character == '\\' then scanBatchCases rest (character :: current) cases depth true true
        else if character == '"' then scanBatchCases rest (character :: current) cases depth false false
        else if character.toNat < 32 then none
        else scanBatchCases rest (character :: current) cases depth true false
      else if jsonWhitespace character then none
      else if character == '"' then scanBatchCases rest (character :: current) cases depth true false
      else if character == '{' || character == '[' then
        scanBatchCases rest (character :: current) cases (depth + 1) false false
      else if character == '}' || character == ']' then
        if depth == 0 then none
        else scanBatchCases rest (character :: current) cases (depth - 1) false false
      else if character == ',' && depth == 0 then
        if current.isEmpty then none
        else scanBatchCases rest [] (current.reverse :: cases) 0 false false
      else scanBatchCases rest (character :: current) cases depth false false

private def arrayCaseBytes? (bytes : ByteArray) : Option (List ByteArray) := do
  let text ← String.fromUTF8? bytes
  match text.toList with
  | '[' :: rest =>
      match rest.reverse with
      | ']' :: reversedBody =>
          let body := reversedBody.reverse
          if body.isEmpty then some []
          else (← scanBatchCases body [] [] 0 false false).map fun characters =>
            String.ofList characters |>.toUTF8
      | _ => none
  | _ => none

private def batchCaseBytes? (bytes : ByteArray) : Option (List ByteArray) := do
  let text ← String.fromUTF8? bytes
  let batchPrefix := "{\"cases\":"
  let batchSuffix := ",\"schema\":\"neco.calculus.observation-batch.v1\"}"
  if !text.startsWith batchPrefix || !text.endsWith batchSuffix then none
  else
    let body := ((text.toList.drop batchPrefix.toList.length).reverse.drop
      batchSuffix.toList.length).reverse
    arrayCaseBytes? (String.ofList body).toUTF8

private def checkCaseBytes (bytes : ByteArray) : Option CaseResult :=
  match NecoLean.Json.decode bytes with
  | .ok value =>
      match NecoLean.Json.encode value with
      | .ok encoded => if encoded == bytes then some (checkCase value) else none
      | .error _ => none
  | .error _ => none

private def uniqueCaseIds : List CaseResult → Bool
  | [] => true
  | item :: remaining =>
      !item.caseId.isEmpty && !(remaining.any fun candidate => candidate.caseId == item.caseId) &&
        uniqueCaseIds remaining

private def checkedBatch (digest : String) (cases : List CaseResult) : CheckResult :=
  if uniqueCaseIds cases then .cases digest cases else .batchInvalid digest

/-- Checks a canonical protocol batch without any file or process effects. -/
def checkBytes (bytes : ByteArray) : CheckResult :=
  let digest := NecoLean.Digest.toHex (NecoLean.Digest.sha256 bytes)
  match NecoLean.Json.decode bytes with
  | .ok (.object fields) =>
      match NecoLean.Json.encode (.object fields),
          jsonObjectWithExactFields? (.object fields) ["cases", "schema"],
          jsonField? fields "cases", jsonStringField? fields "schema" with
      | .ok encoded, some _, some (.array cases), some "neco.calculus.observation-batch.v1" =>
          if encoded == bytes then checkedBatch digest (cases.map checkCase)
          else .batchInvalid digest
      | _, _, _, _ => .batchInvalid digest
  | _ =>
      match batchCaseBytes? bytes with
      | some cases =>
          match cases.mapM checkCaseBytes with
          | some items => checkedBatch digest items
          | none => .batchInvalid digest
      | none => .batchInvalid digest

private def verdictText : Verdict → String
  | .verified => "verified"
  | .refuted => "refuted"
  | .undetermined => "undetermined"
  | .invalid => "invalid"

def resultJson : CheckResult → Value
  | .batchInvalid digest =>
      .object [("batchVerdict", .string "invalid"), ("digest", .string digest)]
  | .cases digest items =>
      .object [
        ("cases", .array (items.map fun item =>
          .object [("caseId", .string item.caseId), ("verdict", .string (verdictText item.verdict))])),
        ("digest", .string digest)
      ]

def resultBytes (result : CheckResult) : ByteArray :=
  match NecoLean.Json.encode (resultJson result) with
  | .ok bytes => bytes
  | .error _ => "{\"batchVerdict\":\"invalid\",\"digest\":\"\"}".toUTF8

end NecoCalculusVerify
