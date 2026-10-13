import NecoCalculusVerify.Semantics
import NecoAlgebraVerify.Soundness

set_option autoImplicit false

namespace NecoCalculusVerify.SurfacePolynomial

private def precedes (left right : SurfaceTerm) : Bool :=
  left.exponentU < right.exponentU ||
    (left.exponentU == right.exponentU && left.exponentV < right.exponentV)

private def mergeTerm (term : SurfaceTerm) : List SurfaceTerm → List SurfaceTerm
  | [] => if term.coefficient == 0 then [] else [term]
  | current :: remaining =>
      if current.exponentU == term.exponentU && current.exponentV == term.exponentV then
        let merged : SurfaceTerm :=
          { coefficient := current.coefficient + term.coefficient
            exponentU := current.exponentU
            exponentV := current.exponentV }
        if merged.coefficient == 0 then remaining else merged :: remaining
      else
        current :: mergeTerm term remaining

private def insertSorted (term : SurfaceTerm) : List SurfaceTerm → List SurfaceTerm
  | [] => [term]
  | current :: remaining =>
      if precedes term current then term :: current :: remaining
      else current :: insertSorted term remaining

def normalize (rawTerms : List SurfaceTerm) : List SurfaceTerm :=
  let merged := rawTerms.foldl (fun terms term => mergeTerm term terms) []
  merged.foldl (fun terms term => insertSorted term terms) []

def scale (scalar : Rat) (poly : List SurfaceTerm) : List SurfaceTerm :=
  poly.map fun term => { term with coefficient := scalar * term.coefficient }

def neg (poly : List SurfaceTerm) : List SurfaceTerm := scale (-1) poly

def add (left right : List SurfaceTerm) : List SurfaceTerm :=
  normalize (left ++ right)

def sub (left right : List SurfaceTerm) : List SurfaceTerm :=
  normalize (left ++ neg right)

private def mulTerm (left right : SurfaceTerm) : SurfaceTerm :=
  { coefficient := left.coefficient * right.coefficient
    exponentU := left.exponentU + right.exponentU
    exponentV := left.exponentV + right.exponentV }

def mul (left right : List SurfaceTerm) : List SurfaceTerm :=
  normalize
    (left.foldl (fun acc lt => acc ++ right.map (mulTerm lt)) [])

def pow : List SurfaceTerm → Nat → List SurfaceTerm
  | _, 0 => [{ coefficient := 1, exponentU := 0, exponentV := 0 }]
  | poly, exponent + 1 => mul (pow poly exponent) poly

def constant (value : Rat) : List SurfaceTerm :=
  if value == 0 then [] else
    [{ coefficient := value, exponentU := 0, exponentV := 0 }]

def variableU : List SurfaceTerm :=
  [{ coefficient := 1, exponentU := 1, exponentV := 0 }]

def variableV : List SurfaceTerm :=
  [{ coefficient := 1, exponentU := 0, exponentV := 1 }]

def maxDegree (terms : List SurfaceTerm) (select : SurfaceTerm → Nat) : Nat :=
  terms.foldl (fun degree term => max degree (select term)) 0

private theorem foldl_add (g : SurfaceTerm → Rat) (terms : List SurfaceTerm) :
    ∀ acc, terms.foldl (fun total term => total + g term) acc =
      acc + terms.foldl (fun total term => total + g term) 0 := by
  induction terms with
  | nil => intro acc; simp
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih (acc + g term), ih (0 + g term)]
      ring

private theorem eval_foldl (terms : List SurfaceTerm) (acc u v : Rat) :
    terms.foldl (fun total term => total + monomial term u v) acc =
      acc + surfaceValue terms u v :=
  foldl_add _ terms acc

theorem value_append (left right : List SurfaceTerm) (u v : Rat) :
    surfaceValue (left ++ right) u v =
      surfaceValue left u v + surfaceValue right u v := by
  unfold surfaceValue
  rw [List.foldl_append, eval_foldl]
  simp [eval_foldl]

private theorem monomial_add_term (left right : SurfaceTerm) (u v : Rat)
    (sameU : left.exponentU = right.exponentU)
    (sameV : left.exponentV = right.exponentV) :
    monomial ⟨left.coefficient + right.coefficient, left.exponentU,
        left.exponentV⟩ u v =
      monomial left u v + monomial right u v := by
  simp only [monomial, sameU, sameV]
  ring

private theorem mergeTerm_eval (term : SurfaceTerm) (acc : List SurfaceTerm)
    (u v : Rat) :
    surfaceValue (mergeTerm term acc) u v =
      monomial term u v + surfaceValue acc u v := by
  induction acc with
  | nil =>
      unfold mergeTerm
      split
      · rename_i hzero
        have hz : term.coefficient = 0 := beq_iff_eq.mp hzero
        simp [surfaceValue, monomial, hz]
      · simp [surfaceValue, List.foldl_cons]
  | cons current remaining ih =>
      unfold mergeTerm
      split
      · rename_i hsame
        have ⟨hu, hv⟩ := Bool.and_eq_true _ _ |>.mp hsame
        have hu' : current.exponentU = term.exponentU := beq_iff_eq.mp hu
        have hv' : current.exponentV = term.exponentV := beq_iff_eq.mp hv
        have hmono := monomial_add_term current term u v hu' hv'
        dsimp only
        split
        · rename_i hmerged
          have hcoef : current.coefficient + term.coefficient = 0 :=
            beq_iff_eq.mp hmerged
          have hzero : monomial current u v + monomial term u v = 0 := by
            rw [← hmono]
            simp [monomial, hcoef]
          simp only [surfaceValue, List.foldl_cons]
          simp only [eval_foldl]
          linarith
        · simp only [surfaceValue, List.foldl_cons]
          simp only [eval_foldl]
          rw [hmono]
          ring
      · simp only [surfaceValue, List.foldl_cons]
        simp only [eval_foldl]
        rw [ih]
        ring

private theorem insertSorted_eval (term : SurfaceTerm) (acc : List SurfaceTerm)
    (u v : Rat) :
    surfaceValue (insertSorted term acc) u v =
      monomial term u v + surfaceValue acc u v := by
  induction acc with
  | nil => simp [insertSorted, surfaceValue, List.foldl_cons]
  | cons current remaining ih =>
      unfold insertSorted
      split
      · simp only [surfaceValue, List.foldl_cons]
        simp only [eval_foldl]
        ring
      · simp only [surfaceValue, List.foldl_cons]
        simp only [eval_foldl]
        rw [ih]
        ring

private theorem mergeAll_eval (rawTerms : List SurfaceTerm) (u v : Rat) :
    ∀ acc, surfaceValue
        (rawTerms.foldl (fun terms term => mergeTerm term terms) acc) u v =
      surfaceValue acc u v + surfaceValue rawTerms u v := by
  induction rawTerms with
  | nil => intro acc; simp [surfaceValue]
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih, mergeTerm_eval]
      simp only [surfaceValue, List.foldl_cons]
      simp only [eval_foldl]
      ring

private theorem sortAll_eval (terms : List SurfaceTerm) (u v : Rat) :
    ∀ acc, surfaceValue
        (terms.foldl (fun acc' term => insertSorted term acc') acc) u v =
      surfaceValue acc u v + surfaceValue terms u v := by
  induction terms with
  | nil => intro acc; simp [surfaceValue]
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih, insertSorted_eval]
      simp only [surfaceValue, List.foldl_cons]
      simp only [eval_foldl]
      ring

theorem normalize_eval (terms : List SurfaceTerm) (u v : Rat) :
    surfaceValue (normalize terms) u v = surfaceValue terms u v := by
  unfold normalize
  rw [sortAll_eval, mergeAll_eval]
  simp [surfaceValue]

theorem add_eval (left right : List SurfaceTerm) (u v : Rat) :
    surfaceValue (add left right) u v =
      surfaceValue left u v + surfaceValue right u v := by
  unfold add
  rw [normalize_eval, value_append]

private theorem scale_foldl (scalar : Rat) (poly : List SurfaceTerm)
    (acc u v : Rat) :
    poly.foldl (fun total term => total +
        monomial { term with coefficient := scalar * term.coefficient } u v) acc =
      acc + scalar *
        poly.foldl (fun total term => total + monomial term u v) 0 := by
  induction poly generalizing acc with
  | nil => simp
  | cons term remaining ih =>
      simp only [List.foldl_cons]
      rw [ih (acc + monomial { term with coefficient := scalar * term.coefficient }
        u v),
        foldl_add (fun term => monomial term u v) remaining
          (0 + monomial term u v)]
      simp only [monomial]
      ring

theorem scale_eval (scalar : Rat) (poly : List SurfaceTerm) (u v : Rat) :
    surfaceValue (scale scalar poly) u v =
      scalar * surfaceValue poly u v := by
  unfold scale surfaceValue
  rw [List.foldl_map]
  rw [scale_foldl scalar poly 0 u v, zero_add]

theorem neg_eval (poly : List SurfaceTerm) (u v : Rat) :
    surfaceValue (neg poly) u v = -surfaceValue poly u v := by
  unfold neg
  rw [scale_eval]
  ring

theorem sub_eval (left right : List SurfaceTerm) (u v : Rat) :
    surfaceValue (sub left right) u v =
      surfaceValue left u v - surfaceValue right u v := by
  unfold sub
  rw [normalize_eval, value_append, neg_eval]
  ring

theorem mulTerm_eval (left right : SurfaceTerm) (u v : Rat) :
    monomial (mulTerm left right) u v =
      monomial left u v * monomial right u v := by
  simp only [monomial, mulTerm]
  rw [pow_add, pow_add]
  ring

private theorem mulTerms_eval (term : SurfaceTerm) (right : List SurfaceTerm)
    (u v : Rat) :
    surfaceValue (right.map (mulTerm term)) u v =
      monomial term u v * surfaceValue right u v := by
  induction right with
  | nil => simp [surfaceValue]
  | cons r remaining ih =>
      simp only [List.map_cons, surfaceValue, List.foldl_cons]
      simp only [eval_foldl]
      rw [mulTerm_eval, ih]
      ring

private theorem mulRaw_eval (left right : List SurfaceTerm) (u v : Rat) :
    ∀ acc, surfaceValue
        (left.foldl (fun acc' lt => acc' ++ right.map (mulTerm lt)) acc) u v =
      surfaceValue acc u v + surfaceValue left u v * surfaceValue right u v := by
  induction left with
  | nil => intro acc; simp [surfaceValue]
  | cons lt remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih]
      simp only [surfaceValue, List.foldl_cons]
      simp only [eval_foldl]
      rw [value_append, mulTerms_eval]
      ring

theorem mul_eval (left right : List SurfaceTerm) (u v : Rat) :
    surfaceValue (mul left right) u v =
      surfaceValue left u v * surfaceValue right u v := by
  unfold mul
  rw [normalize_eval, mulRaw_eval]
  simp [surfaceValue]

theorem pow_eval (poly : List SurfaceTerm) (exponent : Nat) (u v : Rat) :
    surfaceValue (pow poly exponent) u v =
      surfaceValue poly u v ^ exponent := by
  induction exponent with
  | zero => simp [pow, surfaceValue, List.foldl_cons, monomial]
  | succ e ih => simp [pow, mul_eval, ih, pow_succ]

theorem constant_eval (value : Rat) (u v : Rat) :
    surfaceValue (constant value) u v = value := by
  unfold constant
  split
  · rename_i hzero
    have hv : value = 0 := beq_iff_eq.mp hzero
    simp [surfaceValue, hv]
  · simp [surfaceValue, List.foldl_cons, monomial]

theorem variableU_eval (u v : Rat) :
    surfaceValue variableU u v = u := by
  simp [variableU, surfaceValue, List.foldl_cons, monomial]

theorem variableV_eval (u v : Rat) :
    surfaceValue variableV u v = v := by
  simp [variableV, surfaceValue, List.foldl_cons, monomial]

theorem map_sum_mul_left (f : SurfaceTerm → Rat) (r : Rat)
    (l : List SurfaceTerm) :
    r * (l.map f).sum = (l.map fun a => r * f a).sum := by
  induction l with
  | nil => simp
  | cons a rest ih => simp [List.map_cons, List.sum_cons, ih, mul_add]

theorem value_sum (terms : List SurfaceTerm) (u v : Rat) :
    surfaceValue terms u v = (terms.map fun t => monomial t u v).sum := by
  suffices h : ∀ acc, terms.foldl (fun total term => total + monomial term u v) acc =
      acc + (terms.map fun t => monomial t u v).sum by
    unfold surfaceValue
    rw [h 0, zero_add]
  induction terms with
  | nil => intro acc; simp
  | cons t rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, monomial]
      ring

end NecoCalculusVerify.SurfacePolynomial
