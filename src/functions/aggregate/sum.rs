// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! SUM aggregate function

use crate::core::Value;
use crate::functions::{
    AggregateFunction, FunctionDataType, FunctionInfo, FunctionSignature, FunctionType,
};
use octo_determin::decimal::{decimal_add, Decimal};
use octo_determin::{bigint_add, BigInt};

use super::DistinctTracker;

/// Sum state - tracks numeric accumulation type (deterministic domain)
#[derive(Default)]
enum SumState {
    #[default]
    Empty,
    Integer(i64),
    Bigint(BigInt),
    Decimal(Decimal),
    // Float is only for non-deterministic path (Float inputs); we keep it
    // as the final fallback state when Float must be preserved
    NonDetFloat(f64),
}

/// SUM aggregate function
///
/// Returns the sum of all non-NULL values in the specified column.
/// Returns int64 for integer inputs, float64 for floating-point inputs.
#[derive(Default)]
pub struct SumFunction {
    state: SumState,
    distinct_tracker: Option<DistinctTracker>,
}

impl AggregateFunction for SumFunction {
    fn name(&self) -> &str {
        "SUM"
    }

    fn info(&self) -> FunctionInfo {
        FunctionInfo::new(
            "SUM",
            FunctionType::Aggregate,
            "Returns the sum of all non-NULL values in the specified column",
            FunctionSignature::new(
                FunctionDataType::Any, // can return either int64 or float64
                vec![FunctionDataType::Any],
                1,
                1,
            ),
        )
    }

    fn accumulate(&mut self, value: &Value, distinct: bool) {
        // Handle NULL values - SUM ignores NULLs
        if value.is_null() {
            return;
        }

        // Handle DISTINCT case
        if distinct {
            if self.distinct_tracker.is_none() {
                self.distinct_tracker = Some(DistinctTracker::default());
            }
            if !self.distinct_tracker.as_mut().unwrap().check_and_add(value) {
                return; // Already seen this value
            }
        }

        // Extract numeric value
        match value {
            Value::Integer(i) => match &mut self.state {
                SumState::Empty => self.state = SumState::Integer(*i),
                SumState::Integer(sum) => match sum.checked_add(*i) {
                    Some(new_sum) => *sum = new_sum,
                    None => {
                        // Overflow: promote to BigInt
                        let big_sum = BigInt::from(*sum);
                        let big_i = BigInt::from(*i);
                        if let Ok(new_big) = bigint_add(big_sum, big_i) {
                            self.state = SumState::Bigint(new_big);
                        }
                    }
                },
                SumState::Bigint(ref acc) => {
                    let big_i = BigInt::from(*i);
                    if let Ok(new_big) = bigint_add(acc.clone(), big_i) {
                        self.state = SumState::Bigint(new_big);
                    }
                }
                SumState::Decimal(ref acc) => {
                    // Promote Integer + Decimal using decimal_add
                    if let Ok(int_dec) = Decimal::new(*i as i128, 0) {
                        if let Ok(new_dec) = decimal_add(acc, &int_dec) {
                            self.state = SumState::Decimal(new_dec);
                        }
                    }
                }
                SumState::NonDetFloat(sum) => {
                    // Non-deterministic Float path: preserve precision loss
                    *sum += *i as f64;
                }
            },
            Value::Float(f) => match &mut self.state {
                SumState::Empty => self.state = SumState::NonDetFloat(*f),
                SumState::Integer(sum) => {
                    self.state = SumState::NonDetFloat(*sum as f64 + f);
                }
                SumState::Bigint(ref big) => {
                    if let Ok(big_i) = i128::try_from(big.clone()) {
                        self.state = SumState::NonDetFloat(big_i as f64 + f);
                    }
                }
                SumState::Decimal(ref acc) => {
                    // Float + Decimal: promote to NonDetFloat (precision loss acknowledged)
                    let f_acc = acc.mantissa() as f64 * 10f64.powi(-(acc.scale() as i32));
                    self.state = SumState::NonDetFloat(f_acc + f);
                }
                SumState::NonDetFloat(sum) => *sum += f,
            },
            // DFP: convert to f64 then handle (DFP binary FP can't be exactly represented as Decimal)
            Value::Extension(data)
                if data.first().copied()
                    == Some(crate::core::DataType::DeterministicFloat as u8) =>
            {
                if let Some(dfp) = value.as_dfp() {
                    let f = dfp.to_f64();
                    match &mut self.state {
                        SumState::Empty => self.state = SumState::NonDetFloat(f),
                        SumState::Integer(sum) => {
                            self.state = SumState::NonDetFloat(*sum as f64 + f);
                        }
                        SumState::Bigint(ref big) => {
                            if let Ok(big_i) = i128::try_from(big.clone()) {
                                self.state = SumState::NonDetFloat(big_i as f64 + f);
                            }
                        }
                        SumState::Decimal(ref acc) => {
                            let f_acc = acc.mantissa() as f64 * 10f64.powi(-(acc.scale() as i32));
                            self.state = SumState::NonDetFloat(f_acc + f);
                        }
                        SumState::NonDetFloat(sum) => *sum += f,
                    }
                }
            }
            // BIGINT: use arbitrary-precision arithmetic
            Value::Extension(data)
                if data.first().copied() == Some(crate::core::DataType::Bigint as u8) =>
            {
                if let Some(big) = value.as_bigint() {
                    match &mut self.state {
                        SumState::Empty => self.state = SumState::Bigint(big),
                        SumState::Integer(sum) => {
                            let big_sum = BigInt::from(*sum);
                            if let Ok(new_big) = bigint_add(big_sum, big) {
                                self.state = SumState::Bigint(new_big);
                            }
                        }
                        SumState::Bigint(ref acc) => {
                            if let Ok(new_big) = bigint_add(acc.clone(), big) {
                                self.state = SumState::Bigint(new_big);
                            }
                        }
                        SumState::Decimal(ref acc) => {
                            // BigInt + Decimal: convert BigInt to Decimal and add
                            if let Ok(big_i) = i128::try_from(big.clone()) {
                                if let Ok(big_dec) = Decimal::new(big_i, 0) {
                                    if let Ok(new_dec) = decimal_add(acc, &big_dec) {
                                        self.state = SumState::Decimal(new_dec);
                                    }
                                }
                            }
                        }
                        SumState::NonDetFloat(sum) => {
                            if let Ok(i) = i128::try_from(big.clone()) {
                                *sum += i as f64;
                            }
                        }
                    }
                }
            }
            // DECIMAL: use decimal arithmetic (deterministic path)
            Value::Extension(data)
                if data.first().copied() == Some(crate::core::DataType::Decimal as u8) =>
            {
                if let Some(dec) = value.as_decimal() {
                    match &mut self.state {
                        SumState::Empty => self.state = SumState::Decimal(dec),
                        SumState::Integer(sum) => {
                            if let Ok(int_dec) = Decimal::new(*sum as i128, 0) {
                                if let Ok(new_dec) = decimal_add(&int_dec, &dec) {
                                    self.state = SumState::Decimal(new_dec);
                                }
                            }
                        }
                        SumState::Bigint(ref big) => {
                            // BigInt + Decimal: convert BigInt to Decimal and add
                            if let Ok(big_i) = i128::try_from(big.clone()) {
                                if let Ok(big_dec) = Decimal::new(big_i, 0) {
                                    if let Ok(new_dec) = decimal_add(&big_dec, &dec) {
                                        self.state = SumState::Decimal(new_dec);
                                    }
                                }
                            }
                        }
                        SumState::Decimal(ref acc) => {
                            if let Ok(new_dec) = decimal_add(acc, &dec) {
                                self.state = SumState::Decimal(new_dec);
                            }
                        }
                        SumState::NonDetFloat(sum) => {
                            let f = dec.mantissa() as f64 * 10f64.powi(-(dec.scale() as i32));
                            *sum += f;
                        }
                    }
                }
            }
            _ => {} // Ignore non-numeric types
        }
    }

    fn result(&self) -> Value {
        match &self.state {
            SumState::Empty => Value::null_unknown(),
            SumState::Integer(sum) => Value::Integer(*sum),
            SumState::Bigint(big) => Value::bigint(big.clone()),
            SumState::Decimal(dec) => Value::decimal(*dec),
            SumState::NonDetFloat(sum) => Value::Float(*sum),
        }
    }

    fn reset(&mut self) {
        self.state = SumState::Empty;
        self.distinct_tracker = None;
    }

    fn clone_box(&self) -> Box<dyn AggregateFunction> {
        Box::new(SumFunction::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_integers() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(1), false);
        sum.accumulate(&Value::Integer(2), false);
        sum.accumulate(&Value::Integer(3), false);
        assert_eq!(sum.result(), Value::Integer(6));
    }

    #[test]
    fn test_sum_floats() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Float(1.5), false);
        sum.accumulate(&Value::Float(2.5), false);
        sum.accumulate(&Value::Float(3.0), false);
        assert_eq!(sum.result(), Value::Float(7.0));
    }

    #[test]
    fn test_sum_mixed() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(1), false);
        sum.accumulate(&Value::Float(2.5), false);
        sum.accumulate(&Value::Integer(3), false);
        assert_eq!(sum.result(), Value::Float(6.5));
    }

    #[test]
    fn test_sum_ignores_null() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(1), false);
        sum.accumulate(&Value::null_unknown(), false);
        sum.accumulate(&Value::Integer(3), false);
        assert_eq!(sum.result(), Value::Integer(4));
    }

    #[test]
    fn test_sum_distinct() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(1), true);
        sum.accumulate(&Value::Integer(1), true); // duplicate
        sum.accumulate(&Value::Integer(2), true);
        sum.accumulate(&Value::Integer(2), true); // duplicate
        assert_eq!(sum.result(), Value::Integer(3)); // 1 + 2
    }

    #[test]
    fn test_sum_empty() {
        let sum = SumFunction::default();
        assert!(sum.result().is_null());
    }

    #[test]
    fn test_sum_reset() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(1), false);
        sum.accumulate(&Value::Integer(2), false);
        sum.reset();
        assert!(sum.result().is_null());
    }

    #[test]
    fn test_sum_negative() {
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::Integer(-5), false);
        sum.accumulate(&Value::Integer(10), false);
        sum.accumulate(&Value::Integer(-3), false);
        assert_eq!(sum.result(), Value::Integer(2));
    }

    // =========================================================================
    // AC-13: SUM aggregate with BIGINT and DECIMAL
    // =========================================================================

    #[test]
    fn test_ac13_sum_bigint() {
        // SUM of BIGINT values returns BIGINT
        use crate::core::{stoolap_parse_bigint, Value};
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("10").unwrap()), false);
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("20").unwrap()), false);
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("30").unwrap()), false);
        let result = sum.result();
        assert!(matches!(result, Value::Extension(_) if result.data_type() == crate::core::DataType::Bigint));
        assert_eq!(result.as_bigint().map(|b| b.to_string()), Some("60".to_string()));
    }

    #[test]
    fn test_ac13_sum_bigint_negative() {
        // SUM of negative BIGINT values
        use crate::core::{stoolap_parse_bigint, Value};
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("-10").unwrap()), false);
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("20").unwrap()), false);
        sum.accumulate(&Value::bigint(stoolap_parse_bigint("-5").unwrap()), false);
        let result = sum.result();
        assert_eq!(result.as_bigint().map(|b| b.to_string()), Some("5".to_string()));
    }

    #[test]
    fn test_ac13_sum_decimal() {
        // SUM of DECIMAL values returns DECIMAL
        use crate::core::{stoolap_parse_decimal, Value};
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::decimal(stoolap_parse_decimal("10.5").unwrap()), false);
        sum.accumulate(&Value::decimal(stoolap_parse_decimal("20.3").unwrap()), false);
        sum.accumulate(&Value::decimal(stoolap_parse_decimal("5.2").unwrap()), false);
        let result = sum.result();
        assert!(matches!(result, Value::Extension(_) if result.data_type() == crate::core::DataType::Decimal));
        // 10.5 + 20.3 + 5.2 = 36.0
        // Check that the value is correct (scale may be canonicalized)
        let dec = result.as_decimal().unwrap();
        let as_f64 = dec.mantissa() as f64 / 10f64.powi(dec.scale() as i32);
        assert!((as_f64 - 36.0).abs() < 0.001);
    }

    #[test]
    fn test_ac13_sum_integer_promotes_to_bigint() {
        // SUM of integers that overflow i64 should promote to BIGINT
        use crate::core::Value;
        let mut sum = SumFunction::default();
        // Add values that would overflow i64 when summed
        let max_i64 = i64::MAX;
        let half1 = max_i64 / 2 + 1;
        let half2 = max_i64 / 2 + 1;
        sum.accumulate(&Value::Integer(half1), false);
        sum.accumulate(&Value::Integer(half2), false);
        sum.accumulate(&Value::Integer(1), false);
        let result = sum.result();
        // After promotion to BigInt, result should be bigint
        assert!(matches!(result, Value::Extension(_) if result.data_type() == crate::core::DataType::Bigint));
    }

    #[test]
    fn test_ac13_sum_decimal_ignores_null() {
        // SUM ignores NULL values
        use crate::core::{stoolap_parse_decimal, Value};
        let mut sum = SumFunction::default();
        sum.accumulate(&Value::decimal(stoolap_parse_decimal("10.5").unwrap()), false);
        sum.accumulate(&Value::null_unknown(), false);
        sum.accumulate(&Value::decimal(stoolap_parse_decimal("20.3").unwrap()), false);
        let result = sum.result();
        let dec = result.as_decimal().unwrap();
        // 10.5 + 20.3 = 30.8
        assert_eq!(dec.mantissa(), 308);
        assert_eq!(dec.scale(), 1);
    }
}
