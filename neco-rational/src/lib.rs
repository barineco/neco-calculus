#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{BigUint, BigintError, Dyadic, DyadicEnclosure, RawRational, ReducedRational};
use neco_polynomial::{Enclosure, Polynomial, PolynomialFailure};

#[derive(Debug, Eq, PartialEq)]
pub struct RationalFunction {
    numerator: Polynomial,
    denominator: Polynomial,
}

impl RationalFunction {
    pub fn new(numerator: Polynomial, denominator: Polynomial) -> Result<Self, RationalFailure> {
        if is_zero_polynomial(&denominator) {
            return Err(RationalFailure::ZeroDenominatorPolynomial);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub fn numerator(&self) -> &Polynomial {
        &self.numerator
    }

    pub fn denominator(&self) -> &Polynomial {
        &self.denominator
    }

    pub fn evaluate(&self, point: &RealAlgebraic) -> Result<RealAlgebraic, RationalFailure> {
        let denominator = self.denominator.evaluate(point)?;
        if denominator.is_zero() {
            return Err(RationalFailure::UndefinedDomain);
        }
        Ok(self.numerator.evaluate(point)?.div(&denominator)?)
    }

    pub fn evaluate_on(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, RationalFailure> {
        let denominator = self
            .denominator
            .evaluate_enclosure(domain, precision_bits)?;
        if denominator.contains_zero() {
            return Err(RationalFailure::DomainContainsZero);
        }
        let numerator = self.numerator.evaluate_enclosure(domain, precision_bits)?;
        Ok(Enclosure::from_dyadic_enclosure(enclosure_div(
            numerator.as_dyadic_enclosure(),
            denominator.as_dyadic_enclosure(),
            precision_bits,
        )?))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RationalFailure {
    ZeroDenominatorPolynomial,
    UndefinedDomain,
    DomainContainsZero,
    Polynomial(PolynomialFailure),
    Algebra(AlgnumError),
    Arithmetic(BigintError),
}

impl From<PolynomialFailure> for RationalFailure {
    fn from(error: PolynomialFailure) -> Self {
        Self::Polynomial(error)
    }
}

impl From<AlgnumError> for RationalFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Algebra(error)
    }
}

impl From<BigintError> for RationalFailure {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

fn is_zero_polynomial(polynomial: &Polynomial) -> bool {
    polynomial.degree() == 0 && polynomial.coefficients()[0].is_zero()
}

fn enclosure_div(
    numerator: &DyadicEnclosure,
    denominator: &DyadicEnclosure,
    precision_bits: u32,
) -> Result<DyadicEnclosure, RationalFailure> {
    let candidates = [
        dyadic_to_rational(numerator.lower())?.div(&dyadic_to_rational(denominator.lower())?)?,
        dyadic_to_rational(numerator.lower())?.div(&dyadic_to_rational(denominator.upper())?)?,
        dyadic_to_rational(numerator.upper())?.div(&dyadic_to_rational(denominator.lower())?)?,
        dyadic_to_rational(numerator.upper())?.div(&dyadic_to_rational(denominator.upper())?)?,
    ];
    let mut lower = candidates[0].try_clone()?;
    let mut upper = candidates[0].try_clone()?;
    for candidate in candidates.iter().skip(1) {
        if candidate.cmp(&lower) == Ordering::Less {
            lower = candidate.try_clone()?;
        }
        if candidate.cmp(&upper) == Ordering::Greater {
            upper = candidate.try_clone()?;
        }
    }
    Ok(DyadicEnclosure::new(
        lower.dyadic_floor(precision_bits)?,
        upper.dyadic_ceil(precision_bits)?,
    )?)
}

fn dyadic_to_rational(value: &Dyadic) -> Result<ReducedRational, BigintError> {
    let denominator = BigUint::one()?.shl_bits(value.exponent() as usize)?;
    RawRational::new(value.integer().try_clone()?, denominator)
        .reduce()
        .map(|reduction| reduction.into_reduced())
}
