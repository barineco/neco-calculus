import NecoCalculusVerify.RationalPolynomial
import NecoAlgebraVerify.Soundness

set_option autoImplicit false

namespace NecoCalculusVerify.RationalPolynomial

open NecoAlgebraVerify

theorem eval_cons (coefficient : Rat) (p : List Rat) (t : Rat) :
    eval (coefficient :: p) t = coefficient + t * eval p t := rfl

theorem eval_single (a t : Rat) : eval [a] t = a := by simp [eval]

theorem eval_two (a b t : Rat) : eval [a, b] t = a + t * b := by simp [eval]

theorem eval_three (a b c t : Rat) :
    eval [a, b, c] t = a + t * (b + t * c) := by simp [eval]

theorem eval_append (p q : List Rat) (t : Rat) :
    eval (p ++ q) t = eval p t + t ^ p.length * eval q t := by
  induction p with
  | nil => simp [eval]
  | cons a p ih =>
      simp only [List.cons_append, eval_cons, List.length_cons, ih, pow_succ]
      ring

theorem eval_replicate_zero (n : Nat) (t : Rat) :
    eval (List.replicate n 0) t = 0 := by
  induction n with
  | zero => rfl
  | succ n ih => simp [List.replicate_succ, eval_cons, ih]

private theorem mem_takeWhile_pred (p : Rat → Bool) (l : List Rat) (c : Rat)
    (h : c ∈ l.takeWhile p) : p c = true := by
  induction l with
  | nil => simp at h
  | cons a rest ih =>
      simp only [List.takeWhile] at h
      split at h
      · rename_i hp
        rw [List.mem_cons] at h
        cases h with
        | inl heq => subst heq; exact hp
        | inr hmem => exact ih hmem
      · simp at h

private theorem takeWhile_zero_eq (pfx : List Rat)
    (allZero : ∀ c ∈ pfx, (c == 0) = true) :
    pfx = List.replicate pfx.length 0 := by
  induction pfx with
  | nil => rfl
  | cons a rest ih =>
      have ha : a = 0 := beq_iff_eq.mp (allZero a (by simp))
      have hrest := ih (fun c hc => allZero c (List.mem_cons_of_mem a hc))
      rw [ha, List.length_cons, List.replicate_succ, ← hrest]

theorem normalize_eval (p : List Rat) (t : Rat) :
    eval (normalize p) t = eval p t := by
  unfold normalize
  have hsplit : (p.reverse.takeWhile (fun c => c == 0)) ++
      (p.reverse.dropWhile (fun c => c == 0)) = p.reverse :=
    List.takeWhile_append_dropWhile
  have hrev : p =
      (p.reverse.dropWhile (fun c => c == 0)).reverse ++
        (p.reverse.takeWhile (fun c => c == 0)).reverse := by
    conv_lhs => rw [← List.reverse_reverse p, ← hsplit]
    rw [List.reverse_append]
  conv_rhs => rw [hrev]
  have hzeros : (p.reverse.takeWhile (fun c => c == 0)).reverse =
      List.replicate (p.reverse.takeWhile (fun c => c == 0)).length 0 := by
    rw [takeWhile_zero_eq _ (fun c hc => mem_takeWhile_pred _ _ c hc),
      List.reverse_replicate, List.length_replicate]
  rw [eval_append, hzeros, eval_replicate_zero]
  simp

theorem add_eval (p q : List Rat) (t : Rat) :
    eval (add p q) t = eval p t + eval q t := by
  induction p generalizing q with
  | nil => simp [add, eval]
  | cons a p ih =>
      cases q with
      | nil => simp [add, eval]
      | cons b q =>
          simp only [add, eval_cons, ih]
          ring

theorem neg_eval (p : List Rat) (t : Rat) :
    eval (neg p) t = -eval p t := by
  induction p with
  | nil => simp [neg, eval]
  | cons a p ih =>
      unfold neg at ih ⊢
      simp only [List.map_cons, eval_cons]
      rw [ih]
      ring

theorem sub_eval (p q : List Rat) (t : Rat) :
    eval (sub p q) t = eval p t - eval q t := by
  unfold sub
  rw [normalize_eval, add_eval, neg_eval]
  ring

theorem scale_eval (s : Rat) (p : List Rat) (t : Rat) :
    eval (scale s p) t = s * eval p t := by
  induction p with
  | nil => simp [scale, eval]
  | cons a p ih =>
      unfold scale at ih ⊢
      simp only [List.map_cons, eval_cons]
      rw [ih]
      ring

theorem shift_eval (p : List Rat) (t : Rat) :
    eval (shift p) t = t * eval p t := by
  simp [shift, eval_cons]

theorem mul_eval (p q : List Rat) (t : Rat) :
    eval (mul p q) t = eval p t * eval q t := by
  induction p with
  | nil => simp [mul, eval]
  | cons a p ih =>
      simp only [mul, add_eval, scale_eval, shift_eval, eval_cons, ih]
      ring

theorem pow_eval (p : List Rat) (n : Nat) (t : Rat) :
    eval (pow p n) t = eval p t ^ n := by
  induction n with
  | zero => simp [pow, eval]
  | succ n ih => simp [pow, mul_eval, ih, pow_succ]

theorem constant_eval (c : Rat) (t : Rat) :
    eval (constant c) t = c := by
  unfold constant
  cases h : c == 0 with
  | true =>
      have hc : c = 0 := beq_iff_eq.mp h
      simp [hc, eval]
  | false => simp [eval_cons, eval]

theorem range_contains (p : List Rat) (domain : NecoAlgebraVerify.Interval) (t : Rat)
    (contained : domain.Contains t) :
    (range p domain).Contains (eval p t) := by
  induction p with
  | nil =>
      unfold range eval NecoAlgebraVerify.Interval.zero
      exact ⟨le_rfl, le_rfl⟩
  | cons a rest ih =>
      unfold range eval
      exact NecoAlgebraVerify.Interval.add_contains (NecoAlgebraVerify.Interval.constant_contains a)
        (NecoAlgebraVerify.Interval.mul_contains contained ih)

theorem nonpositiveOn_sound (num den : List Rat) (lo hi : Rat)
    (check : nonpositiveOn num den lo hi = true) (t : Rat)
    (contained : lo ≤ t ∧ t ≤ hi) :
    eval den t ≠ 0 ∧ eval num t ≤ 0 := by
  unfold nonpositiveOn at check
  rw [Bool.and_eq_true] at check
  obtain ⟨hzero, hupper⟩ := check
  have hexcludes : (range den { lower := lo, upper := hi }).ExcludesZero :=
    of_decide_eq_true hzero
  refine ⟨fun heval => hexcludes
    (heval ▸ range_contains den { lower := lo, upper := hi } t contained), ?_⟩
  exact le_trans (range_contains num { lower := lo, upper := hi } t contained).2
    (of_decide_eq_true hupper)

theorem nonpositiveSub_sound (num den : List Rat) (depth : Nat) (lo hi : Rat)
    (check : nonpositiveSub num den lo hi depth = true) (t : Rat)
    (contained : lo ≤ t ∧ t ≤ hi) :
    eval den t ≠ 0 ∧ eval num t ≤ 0 := by
  induction depth generalizing lo hi with
  | zero =>
      unfold nonpositiveSub at check
      exact nonpositiveOn_sound _ _ _ _ check t contained
  | succ depth ih =>
      unfold nonpositiveSub at check
      dsimp only at check
      rw [Bool.or_eq_true] at check
      cases check with
      | inl hdirect => exact nonpositiveOn_sound _ _ _ _ hdirect t contained
      | inr hsplit =>
          rw [Bool.and_eq_true] at hsplit
          by_cases hside : t ≤ (lo + hi) / 2
          · exact ih lo ((lo + hi) / 2) hsplit.1 ⟨contained.1, hside⟩
          · exact ih ((lo + hi) / 2) hi hsplit.2
              ⟨le_of_lt (lt_of_not_ge hside), contained.2⟩

theorem range_valid (p : List Rat) (domain : NecoAlgebraVerify.Interval) :
    (range p domain).Valid := by
  induction p with
  | nil => exact NecoAlgebraVerify.Interval.zero_valid
  | cons a rest ih =>
      unfold range
      exact NecoAlgebraVerify.Interval.add_valid
        (NecoAlgebraVerify.Interval.constant_valid a)
        (NecoAlgebraVerify.Interval.mul_valid _ _)

theorem rationalRange_sound (num den : List Rat) (lo hi : Rat)
    (range : NecoAlgebraVerify.Interval)
    (check : rationalRange num den lo hi = some range) (t : Rat)
    (contained : lo ≤ t ∧ t ≤ hi) :
    eval den t ≠ 0 ∧ range.Contains (eval num t / eval den t) := by
  unfold rationalRange at check
  dsimp only at check
  split at check
  · rename_i hden
    simp only [Option.some.injEq] at check
    subst check
    have hdenContains := range_contains den { lower := lo, upper := hi } t contained
    refine ⟨fun heval => hden (heval ▸ hdenContains), ?_⟩
    exact NecoAlgebraVerify.Interval.div_contains (range_valid num _)
      (range_valid den _) hden
      (range_contains num { lower := lo, upper := hi } t contained)
      hdenContains
  · simp at check

private theorem square_le_maxAbs_square (value : Rat) (interval : NecoAlgebraVerify.Interval)
    (contained : interval.Contains value) :
    value * value ≤ NecoAlgebraVerify.Interval.maxAbs interval *
      NecoAlgebraVerify.Interval.maxAbs interval := by
  have h := mul_self_le_mul_self (abs_nonneg value)
    (NecoAlgebraVerify.Interval.abs_le_maxAbs interval value contained)
  rwa [← abs_mul, abs_of_nonneg (mul_self_nonneg value)] at h

theorem squaredSumWithinOn_sound (nx dx ny dy nz dz : List Rat) (bound lo hi : Rat)
    (check : squaredSumWithinOn nx dx ny dy nz dz bound lo hi = true) (t : Rat)
    (contained : lo ≤ t ∧ t ≤ hi) :
    eval dx t ≠ 0 ∧ eval dy t ≠ 0 ∧ eval dz t ≠ 0 ∧
      eval nx t / eval dx t * (eval nx t / eval dx t) +
        eval ny t / eval dy t * (eval ny t / eval dy t) +
        eval nz t / eval dz t * (eval nz t / eval dz t) ≤ bound := by
  unfold squaredSumWithinOn at check
  split at check
  · rename_i rx ry rz hrx hry hrz
    have hsum := of_decide_eq_true check
    have hx := rationalRange_sound _ _ _ _ rx hrx t contained
    have hy := rationalRange_sound _ _ _ _ ry hry t contained
    have hz := rationalRange_sound _ _ _ _ rz hrz t contained
    refine ⟨hx.1, hy.1, hz.1, ?_⟩
    have sx := square_le_maxAbs_square _ _ hx.2
    have sy := square_le_maxAbs_square _ _ hy.2
    have sz := square_le_maxAbs_square _ _ hz.2
    linarith
  · simp at check

theorem squaredSumWithinSub_sound (nx dx ny dy nz dz : List Rat) (bound : Rat)
    (depth : Nat) (lo hi : Rat)
    (check : squaredSumWithinSub nx dx ny dy nz dz bound lo hi depth = true) (t : Rat)
    (contained : lo ≤ t ∧ t ≤ hi) :
    eval dx t ≠ 0 ∧ eval dy t ≠ 0 ∧ eval dz t ≠ 0 ∧
      eval nx t / eval dx t * (eval nx t / eval dx t) +
        eval ny t / eval dy t * (eval ny t / eval dy t) +
        eval nz t / eval dz t * (eval nz t / eval dz t) ≤ bound := by
  induction depth generalizing lo hi with
  | zero =>
      unfold squaredSumWithinSub at check
      exact squaredSumWithinOn_sound _ _ _ _ _ _ _ _ _ check t contained
  | succ depth ih =>
      unfold squaredSumWithinSub at check
      dsimp only at check
      rw [Bool.or_eq_true] at check
      cases check with
      | inl hdirect =>
          exact squaredSumWithinOn_sound _ _ _ _ _ _ _ _ _ hdirect t contained
      | inr hsplit =>
          rw [Bool.and_eq_true] at hsplit
          by_cases hside : t ≤ (lo + hi) / 2
          · exact ih lo ((lo + hi) / 2) hsplit.1 ⟨contained.1, hside⟩
          · exact ih ((lo + hi) / 2) hi hsplit.2
              ⟨le_of_lt (lt_of_not_ge hside), contained.2⟩

private theorem composedSurfaceTerm_eval (term : SurfaceTerm)
    (uN uD vN vD : List Rat) (degreeU degreeV : Nat) (t : Rat) :
    eval (scale term.coefficient
      (mul (pow uN term.exponentU)
        (mul (pow vN term.exponentV)
          (mul (pow uD (degreeU - term.exponentU))
            (pow vD (degreeV - term.exponentV)))))) t =
    term.coefficient * eval uN t ^ term.exponentU *
      eval vN t ^ term.exponentV *
      eval uD t ^ (degreeU - term.exponentU) *
      eval vD t ^ (degreeV - term.exponentV) := by
  rw [scale_eval, mul_eval, mul_eval, mul_eval,
    pow_eval, pow_eval, pow_eval, pow_eval]
  ring

private theorem composedSurfaceFoldl_eval (terms : List SurfaceTerm)
    (uN uD vN vD : List Rat) (degreeU degreeV : Nat) (t : Rat) :
    ∀ acc, eval (terms.foldl (fun acc term =>
        add acc (scale term.coefficient
          (mul (pow uN term.exponentU)
            (mul (pow vN term.exponentV)
              (mul (pow uD (degreeU - term.exponentU))
                (pow vD (degreeV - term.exponentV))))))) acc) t =
      eval acc t + (terms.map fun term =>
        term.coefficient * eval uN t ^ term.exponentU *
          eval vN t ^ term.exponentV *
          eval uD t ^ (degreeU - term.exponentU) *
          eval vD t ^ (degreeV - term.exponentV)).sum := by
  induction terms with
  | nil => intro acc; simp
  | cons term rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, add_eval, composedSurfaceTerm_eval]
      ring

theorem composeSurface_eval (terms : List SurfaceTerm) (uN uD vN vD : List Rat)
    (t : Rat) (huD : eval uD t ≠ 0) (hvD : eval vD t ≠ 0) :
    eval (composeSurface terms uN uD vN vD).2 t ≠ 0 ∧
      eval (composeSurface terms uN uD vN vD).1 t /
          eval (composeSurface terms uN uD vN vD).2 t =
        surfaceValue terms (eval uN t / eval uD t) (eval vN t / eval vD t) := by
  unfold composeSurface
  dsimp only
  have hfold := composedSurfaceFoldl_eval terms uN uD vN vD
    (SurfacePolynomial.maxDegree terms (·.exponentU))
    (SurfacePolynomial.maxDegree terms (·.exponentV)) t []
  set degreeU := SurfacePolynomial.maxDegree terms (·.exponentU)
  set degreeV := SurfacePolynomial.maxDegree terms (·.exponentV)
  set uNv := eval uN t
  set uDv := eval uD t
  set vNv := eval vN t
  set vDv := eval vD t
  rw [normalize_eval, mul_eval, pow_eval, pow_eval]
  have hdU : uDv ^ degreeU ≠ 0 := pow_ne_zero _ huD
  have hdV : vDv ^ degreeV ≠ 0 := pow_ne_zero _ hvD
  have hden : uDv ^ degreeU * vDv ^ degreeV ≠ 0 := mul_ne_zero hdU hdV
  refine ⟨hden, ?_⟩
  rw [normalize_eval]
  have hboundU : ∀ s ∈ terms, s.exponentU ≤ degreeU := fun s hs =>
    foldl_max_ge_mem (fun t => t.exponentU) terms s hs 0
  have hboundV : ∀ s ∈ terms, s.exponentV ≤ degreeV := fun s hs =>
    foldl_max_ge_mem (fun t => t.exponentV) terms s hs 0
  rw [hfold]
  have hz : eval ([] : List Rat) t = 0 := rfl
  rw [hz, zero_add, SurfacePolynomial.value_sum]
  have hmul := SurfacePolynomial.map_sum_mul_left (fun s => monomial s (uNv / uDv)
    (vNv / vDv)) (uDv ^ degreeU * vDv ^ degreeV) terms
  rw [div_eq_iff hden]
  rw [mul_comm ((terms.map _).sum) (uDv ^ degreeU * vDv ^ degreeV), hmul]
  apply congrArg List.sum
  apply List.map_congr_left
  intro s hs
  have hbU := hboundU s hs
  have hbV := hboundV s hs
  have hpowU : uDv ^ degreeU = uDv ^ s.exponentU * uDv ^ (degreeU - s.exponentU) := by
    rw [← pow_add, Nat.add_sub_cancel' hbU]
  have hpowV : vDv ^ degreeV = vDv ^ s.exponentV * vDv ^ (degreeV - s.exponentV) := by
    rw [← pow_add, Nat.add_sub_cancel' hbV]
  simp only [monomial]
  rw [hpowU, hpowV, div_pow, div_pow]
  have hdu : uDv ^ s.exponentU ≠ 0 := pow_ne_zero _ huD
  have hdv : vDv ^ s.exponentV ≠ 0 := pow_ne_zero _ hvD
  field_simp

end NecoCalculusVerify.RationalPolynomial
