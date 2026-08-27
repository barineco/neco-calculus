#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{BigInt, BigintError, Dyadic, DyadicEnclosure};

pub const MAX_COEFFICIENTS: usize = 256;
pub const MAX_DERIVATIVE_ORDER: usize = 64;
pub const MAX_PRECISION_BITS: u32 = 256;

#[derive(Debug, Eq, PartialEq)]
pub struct Polynomial {
    coefficients: Vec<RealAlgebraic>,
}

impl Polynomial {
    pub fn from_coefficients(
        mut coefficients: Vec<RealAlgebraic>,
    ) -> Result<Self, PolynomialFailure> {
        if coefficients.is_empty() {
            return Err(PolynomialFailure::EmptyCoefficients);
        }
        if coefficients.len() > MAX_COEFFICIENTS {
            return Err(ResourceFailure::CoefficientLimitExceeded {
                requested: coefficients.len(),
                maximum: MAX_COEFFICIENTS,
            }
            .into());
        }
        while coefficients.len() > 1 && coefficients.last().is_some_and(RealAlgebraic::is_zero) {
            coefficients.pop();
        }
        Ok(Self { coefficients })
    }

    pub fn zero() -> Result<Self, PolynomialFailure> {
        Self::constant(algebraic_integer(0)?)
    }

    pub fn constant(value: RealAlgebraic) -> Result<Self, PolynomialFailure> {
        Self::from_coefficients(alloc::vec![value])
    }

    pub fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }

    pub fn coefficients(&self) -> &[RealAlgebraic] {
        &self.coefficients
    }

    pub fn try_clone(&self) -> Result<Self, PolynomialFailure> {
        let mut coefficients = Vec::new();
        for coefficient in &self.coefficients {
            coefficients.push(coefficient.try_clone()?);
        }
        Ok(Self { coefficients })
    }

    pub fn evaluate(&self, point: &RealAlgebraic) -> Result<RealAlgebraic, PolynomialFailure> {
        let mut coefficients = self.coefficients.iter().rev();
        let first = coefficients
            .next()
            .ok_or(PolynomialFailure::EmptyCoefficients)?
            .try_clone()?;
        let mut value = first;
        for coefficient in coefficients {
            value = value.mul(point)?.add(coefficient)?;
        }
        Ok(value)
    }

    pub fn derivative(&self, order: DerivativeOrder) -> Result<Self, PolynomialFailure> {
        validate_derivative_order(order)?;
        let mut result = self.try_clone()?;
        for _ in 0..order.get() {
            result = result.derivative_once()?;
        }
        Ok(result)
    }

    pub fn evaluate_enclosure(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, PolynomialFailure> {
        validate_precision(precision_bits)?;
        let mut coefficients = self.coefficients.iter().rev();
        let first = coefficients
            .next()
            .ok_or(PolynomialFailure::EmptyCoefficients)?
            .enclose(precision_bits)?
            .enclosure()
            .try_clone()?;
        let mut value = first;
        for coefficient in coefficients {
            let coefficient = coefficient
                .enclose(precision_bits)?
                .enclosure()
                .try_clone()?;
            value = enclosure_add(
                &enclosure_mul(&value, domain.as_dyadic_enclosure())?,
                &coefficient,
            )?;
        }
        Ok(Enclosure::from_dyadic_enclosure(value))
    }

    fn derivative_once(&self) -> Result<Self, PolynomialFailure> {
        if self.coefficients.len() <= 1 {
            return Self::zero();
        }
        let mut coefficients = Vec::new();
        for (degree, coefficient) in self.coefficients.iter().enumerate().skip(1) {
            coefficients.push(coefficient.mul(&algebraic_integer(degree)?)?);
        }
        Self::from_coefficients(coefficients)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DerivativeOrder(usize);

impl DerivativeOrder {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Enclosure {
    value: DyadicEnclosure,
}

impl Enclosure {
    pub fn new(lower: Dyadic, upper: Dyadic) -> Result<Self, IntervalFailure> {
        Ok(Self {
            value: DyadicEnclosure::new(lower, upper)?,
        })
    }

    pub fn from_dyadic_enclosure(value: DyadicEnclosure) -> Self {
        Self { value }
    }

    pub fn as_dyadic_enclosure(&self) -> &DyadicEnclosure {
        &self.value
    }

    pub fn lower(&self) -> &Dyadic {
        self.value.lower()
    }

    pub fn upper(&self) -> &Dyadic {
        self.value.upper()
    }

    pub fn width(&self) -> Result<Dyadic, IntervalFailure> {
        Ok(self.value.width()?)
    }

    pub fn midpoint(&self) -> Result<Dyadic, IntervalFailure> {
        Ok(self.value.midpoint()?)
    }

    pub fn contains(&self, value: &Dyadic) -> bool {
        self.value.contains_dyadic(value)
    }

    pub fn contains_zero(&self) -> bool {
        self.contains(&Dyadic::new(BigInt::zero(), 0))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum PolynomialFailure {
    EmptyCoefficients,
    Algebra(AlgnumError),
    Arithmetic(BigintError),
    Interval(IntervalFailure),
    Resource(ResourceFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceFailure {
    CoefficientLimitExceeded { requested: usize, maximum: usize },
    DerivativeOrderLimitExceeded { requested: usize, maximum: usize },
    PrecisionLimitExceeded { requested: u32, maximum: u32 },
}

#[derive(Debug, Eq, PartialEq)]
pub enum IntervalFailure {
    Arithmetic(BigintError),
}

impl From<AlgnumError> for PolynomialFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Algebra(error)
    }
}

impl From<BigintError> for PolynomialFailure {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<IntervalFailure> for PolynomialFailure {
    fn from(error: IntervalFailure) -> Self {
        Self::Interval(error)
    }
}

impl From<ResourceFailure> for PolynomialFailure {
    fn from(error: ResourceFailure) -> Self {
        Self::Resource(error)
    }
}

impl From<BigintError> for IntervalFailure {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

fn algebraic_integer(value: usize) -> Result<RealAlgebraic, PolynomialFailure> {
    Ok(RealAlgebraic::from_integer(BigInt::try_from(value)?)?)
}

fn validate_derivative_order(order: DerivativeOrder) -> Result<(), PolynomialFailure> {
    if order.get() > MAX_DERIVATIVE_ORDER {
        return Err(ResourceFailure::DerivativeOrderLimitExceeded {
            requested: order.get(),
            maximum: MAX_DERIVATIVE_ORDER,
        }
        .into());
    }
    Ok(())
}

fn validate_precision(precision_bits: u32) -> Result<(), PolynomialFailure> {
    if precision_bits > MAX_PRECISION_BITS {
        return Err(ResourceFailure::PrecisionLimitExceeded {
            requested: precision_bits,
            maximum: MAX_PRECISION_BITS,
        }
        .into());
    }
    Ok(())
}

fn enclosure_add(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
) -> Result<DyadicEnclosure, PolynomialFailure> {
    Ok(DyadicEnclosure::new(
        left.lower().add(right.lower())?,
        left.upper().add(right.upper())?,
    )?)
}

fn enclosure_mul(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
) -> Result<DyadicEnclosure, PolynomialFailure> {
    let candidates = [
        left.lower().mul(right.lower())?,
        left.lower().mul(right.upper())?,
        left.upper().mul(right.lower())?,
        left.upper().mul(right.upper())?,
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
    Ok(DyadicEnclosure::new(lower, upper)?)
}
