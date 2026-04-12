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

// Tests for the Compiled Expression VM

use std::sync::Arc;

use crate::common::SmartString;

use crate::common::CompactArc;

use super::compiler::{CompileContext, ExprCompiler};
use super::ops::{CompiledPattern, Op};
use super::program::Program;
use super::vm::{ExecuteContext, ExprVM};
use crate::core::{DataType, Value, ValueSet};
use crate::Row;

#[test]
fn test_simple_load_and_compare() {
    let mut vm = ExprVM::new();

    // col[0] > 5
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::Gt,
        Op::Return,
    ]);

    // True case
    let row = Row::from_values(vec![Value::Integer(10)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // False case
    let row = Row::from_values(vec![Value::Integer(3)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_null_comparison() {
    let mut vm = ExprVM::new();

    // col[0] > 5 with NULL col
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::Gt,
        Op::Return,
    ]);

    let row = Row::from_values(vec![Value::Null(crate::core::DataType::Integer)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_and_short_circuit() {
    let mut vm = ExprVM::new();

    // col[0] > 5 AND col[1] < 10
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::Gt,
        Op::And(10), // Jump to position 10 if false
        Op::LoadColumn(1),
        Op::LoadConst(Value::Integer(10)),
        Op::Lt,
        Op::AndFinalize,
        Op::Return,
        Op::Nop,                              // Position 9
        Op::LoadConst(Value::Boolean(false)), // Position 10
        Op::Return,
    ]);

    // Both true
    let row = Row::from_values(vec![Value::Integer(10), Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // First false (short circuit)
    let row = Row::from_values(vec![Value::Integer(3), Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));

    // First true, second false
    let row = Row::from_values(vec![Value::Integer(10), Value::Integer(15)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_or_short_circuit() {
    let mut vm = ExprVM::new();

    // col[0] < 5 OR col[1] > 10
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::Lt,
        Op::Or(10), // Jump to position 10 if true
        Op::LoadColumn(1),
        Op::LoadConst(Value::Integer(10)),
        Op::Gt,
        Op::OrFinalize,
        Op::Return,
        Op::Nop,                             // Position 9
        Op::LoadConst(Value::Boolean(true)), // Position 10
        Op::Return,
    ]);

    // First true (short circuit)
    let row = Row::from_values(vec![Value::Integer(3), Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // First false, second true
    let row = Row::from_values(vec![Value::Integer(10), Value::Integer(15)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // Both false
    let row = Row::from_values(vec![Value::Integer(10), Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_arithmetic() {
    let mut vm = ExprVM::new();

    // col[0] + col[1] * 2
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadColumn(1),
        Op::LoadConst(Value::Integer(2)),
        Op::Mul,
        Op::Add,
        Op::Return,
    ]);

    let row = Row::from_values(vec![Value::Integer(5), Value::Integer(3)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Integer(11)); // 5 + 3*2 = 11
}

#[test]
fn test_in_set() {
    let mut vm = ExprVM::new();

    let set: ValueSet = [Value::Integer(1), Value::Integer(2), Value::Integer(3)]
        .into_iter()
        .collect();

    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::InSet(CompactArc::new(set), false),
        Op::Return,
    ]);

    // In set
    let row = Row::from_values(vec![Value::Integer(2)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // Not in set
    let row = Row::from_values(vec![Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_between() {
    let mut vm = ExprVM::new();

    // col[0] BETWEEN 5 AND 10
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::LoadConst(Value::Integer(10)),
        Op::Between,
        Op::Return,
    ]);

    // In range
    let row = Row::from_values(vec![Value::Integer(7)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // Below range
    let row = Row::from_values(vec![Value::Integer(3)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));

    // Above range
    let row = Row::from_values(vec![Value::Integer(15)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_like_pattern() {
    let mut vm = ExprVM::new();

    // col[0] LIKE 'test%'
    let pattern = CompiledPattern::compile("test%", false);
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::Like(Arc::new(pattern), false),
        Op::Return,
    ]);

    // Match
    let row = Row::from_values(vec![Value::Text(SmartString::from("testing"))]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // No match
    let row = Row::from_values(vec![Value::Text(SmartString::from("other"))]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_is_null() {
    let mut vm = ExprVM::new();

    // col[0] IS NULL
    let program = Program::new(vec![Op::LoadColumn(0), Op::IsNull, Op::Return]);

    // Is null
    let row = Row::from_values(vec![Value::Null(crate::core::DataType::Integer)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // Not null
    let row = Row::from_values(vec![Value::Integer(5)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_coalesce() {
    let mut vm = ExprVM::new();

    // COALESCE(col[0], col[1], 'default')
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadColumn(1),
        Op::LoadConst(Value::Text(SmartString::from("default"))),
        Op::Coalesce(3),
        Op::Return,
    ]);

    // First non-null
    let row = Row::from_values(vec![
        Value::Text(SmartString::from("first")),
        Value::Text(SmartString::from("second")),
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Text(SmartString::from("first")));

    // Second non-null
    let row = Row::from_values(vec![
        Value::Null(crate::core::DataType::Text),
        Value::Text(SmartString::from("second")),
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Text(SmartString::from("second")));

    // Default
    let row = Row::from_values(vec![
        Value::Null(crate::core::DataType::Text),
        Value::Null(crate::core::DataType::Text),
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Text(SmartString::from("default")));
}

#[test]
fn test_join_context() {
    let mut vm = ExprVM::new();

    // row1.col[0] = row2.col[0]
    let program = Program::new(vec![
        Op::LoadColumn(0),  // From first row
        Op::LoadColumn2(0), // From second row
        Op::Eq,
        Op::Return,
    ]);

    let row1 = Row::from_values(vec![Value::Integer(5)]);
    let row2 = Row::from_values(vec![Value::Integer(5)]);
    let ctx = ExecuteContext::for_join(&row1, &row2);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    let row1 = Row::from_values(vec![Value::Integer(5)]);
    let row2 = Row::from_values(vec![Value::Integer(10)]);
    let ctx = ExecuteContext::for_join(&row1, &row2);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_parameters() {
    let mut vm = ExprVM::new();

    // col[0] = $1
    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadParam(0),
        Op::Eq,
        Op::Return,
    ]);

    let row = Row::from_values(vec![Value::Integer(42)]);
    let params = vec![Value::Integer(42)];
    let ctx = ExecuteContext::new(&row).with_params(&params);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    let params = vec![Value::Integer(100)];
    let ctx = ExecuteContext::new(&row).with_params(&params);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_execute_bool() {
    let mut vm = ExprVM::new();

    let program = Program::new(vec![
        Op::LoadColumn(0),
        Op::LoadConst(Value::Integer(5)),
        Op::Gt,
        Op::Return,
    ]);

    // True
    let row = Row::from_values(vec![Value::Integer(10)]);
    let ctx = ExecuteContext::new(&row);
    assert!(vm.execute_bool(&program, &ctx));

    // False
    let row = Row::from_values(vec![Value::Integer(3)]);
    let ctx = ExecuteContext::new(&row);
    assert!(!vm.execute_bool(&program, &ctx));

    // NULL -> false
    let row = Row::from_values(vec![Value::Null(crate::core::DataType::Integer)]);
    let ctx = ExecuteContext::new(&row);
    assert!(!vm.execute_bool(&program, &ctx));
}

// ============================================================================
// Compiler Tests
// ============================================================================

#[test]
fn test_compiler_simple_expression() {
    use crate::parser::ast::*;
    use crate::parser::token::{Position, Token, TokenType};

    let columns = vec!["a".to_string(), "b".to_string()];
    let ctx = CompileContext::with_global_registry(&columns);
    let compiler = ExprCompiler::new(&ctx);

    fn make_token() -> Token {
        Token {
            token_type: TokenType::Integer,
            literal: "1".into(),
            position: Position {
                offset: 0,
                line: 1,
                column: 1,
            },
            quoted: false,
        }
    }

    // a > 5
    let expr = Expression::Infix(InfixExpression {
        token: make_token(),
        left: Box::new(Expression::Identifier(Identifier::new(
            make_token(),
            "a".to_string(),
        ))),
        operator: ">".into(),
        op_type: InfixOperator::GreaterThan,
        right: Box::new(Expression::IntegerLiteral(IntegerLiteral {
            token: make_token(),
            value: 5,
        })),
    });

    let program = compiler.compile(&expr).unwrap();

    // Execute
    let mut vm = ExprVM::new();
    let row = Row::from_values(vec![Value::Integer(10), Value::Integer(20)]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_compiled_pattern_prefix() {
    let pattern = CompiledPattern::compile("test%", false);
    assert!(pattern.matches("testing", false));
    assert!(pattern.matches("test", false));
    assert!(!pattern.matches("atest", false));
}

#[test]
fn test_compiled_pattern_suffix() {
    let pattern = CompiledPattern::compile("%test", false);
    assert!(pattern.matches("mytest", false));
    assert!(pattern.matches("test", false));
    assert!(!pattern.matches("testa", false));
}

#[test]
fn test_compiled_pattern_contains() {
    let pattern = CompiledPattern::compile("%test%", false);
    assert!(pattern.matches("mytesting", false));
    assert!(pattern.matches("test", false));
    assert!(pattern.matches("atest", false));
    assert!(!pattern.matches("other", false));
}

#[test]
fn test_compiled_pattern_case_insensitive() {
    let pattern = CompiledPattern::compile("TEST%", true);
    assert!(pattern.matches("testing", true));
    assert!(pattern.matches("TESTING", true));
    assert!(pattern.matches("TeStInG", true));
}

// =========================================================================
// AC-6: Cross-type comparison (INTEGER vs BIGINT vs DECIMAL) per RFC-0202-A
// =========================================================================

#[test]
fn test_ac6_integer_bigint_cross_compare() {
    // AC-6: INTEGER vs BIGINT cross-type comparison via as_float64
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // INTEGER(10) > BIGINT(5)
    let program = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("5").unwrap())),
        Op::Gt,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // INTEGER(5) < BIGINT(10)
    let program2 = Program::new(vec![
        Op::LoadConst(Value::Integer(5)),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::Lt,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Boolean(true));

    // INTEGER(10) = BIGINT(10)
    let program3 = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::Eq,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert_eq!(result3, Value::Boolean(true));
}

#[test]
fn test_ac6_integer_decimal_cross_compare() {
    // AC-6: INTEGER vs DECIMAL cross-type comparison
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // INTEGER(10) > DECIMAL(5.5)
    let program = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("5.5").unwrap())),
        Op::Gt,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // INTEGER(5) < DECIMAL(10.1)
    let program2 = Program::new(vec![
        Op::LoadConst(Value::Integer(5)),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.1").unwrap())),
        Op::Lt,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Boolean(true));

    // INTEGER(10) = DECIMAL(10.0)
    let program3 = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.0").unwrap())),
        Op::Eq,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert_eq!(result3, Value::Boolean(true));
}

#[test]
fn test_ac6_bigint_decimal_cross_compare() {
    // AC-6: BIGINT vs DECIMAL cross-type comparison
    use crate::core::{stoolap_parse_bigint, stoolap_parse_decimal};
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT(100) > DECIMAL(50.5)
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("100").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("50.5").unwrap())),
        Op::Gt,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Boolean(true));

    // BIGINT(50) < DECIMAL(50.5)
    let program2 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("50").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("50.5").unwrap())),
        Op::Lt,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Boolean(true));

    // BIGINT(100) = DECIMAL(100.0)
    let program3 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("100").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("100.0").unwrap())),
        Op::Eq,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert_eq!(result3, Value::Boolean(true));
}

// =========================================================================
// AC-16: NULL handling for BIGINT and DECIMAL
// =========================================================================

#[test]
fn test_ac16_bigint_null_handling() {
    // AC-16: NULL handling for BIGINT arithmetic
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT + NULL = NULL
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadNull(DataType::Bigint),
        Op::BigintAdd,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert!(result.is_null());

    // NULL + BIGINT = NULL
    let program2 = Program::new(vec![
        Op::LoadNull(DataType::Bigint),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::BigintAdd,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert!(result2.is_null());

    // BIGINT cmp NULL = NULL
    let program3 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadNull(DataType::Bigint),
        Op::BigintCmp,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert!(result3.is_null());
}

#[test]
fn test_ac16_decimal_null_handling() {
    // AC-16: NULL handling for DECIMAL arithmetic
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // DECIMAL + NULL = NULL
    let program = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::LoadNull(DataType::Decimal),
        Op::DecimalAdd,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert!(result.is_null());

    // NULL + DECIMAL = NULL
    let program2 = Program::new(vec![
        Op::LoadNull(DataType::Decimal),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::DecimalAdd,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert!(result2.is_null());

    // DECIMAL cmp NULL = NULL
    let program3 = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::LoadNull(DataType::Decimal),
        Op::DecimalCmp,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert!(result3.is_null());
}

// =========================================================================
// AC-17: Division by zero checks for BIGINT and DECIMAL
// =========================================================================

#[test]
fn test_ac17_bigint_division_by_zero() {
    // AC-17: BIGINT division by zero returns DivisionByZero error
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT(10) / 0 -> Error::DivisionByZero
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("0").unwrap())),
        Op::BigintDiv,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::core::Error::DivisionByZero));

    // BIGINT(10) % 0 -> Error::DivisionByZero
    let program2 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("0").unwrap())),
        Op::BigintMod,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2);
    assert!(result2.is_err());
    assert!(matches!(result2.unwrap_err(), crate::core::Error::DivisionByZero));
}

#[test]
fn test_ac17_decimal_division_by_zero() {
    // AC-17: DECIMAL division by zero returns DivisionByZero error
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // DECIMAL(10.5) / DECIMAL(0) -> Error::DivisionByZero
    let program = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("0").unwrap())),
        Op::DecimalDiv,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::core::Error::DivisionByZero));
}

#[test]
fn test_ac17_integer_division_by_zero_returns_null() {
    // AC-17: Integer division by zero returns NULL (not error)
    let row = Row::new();
    let mut vm = ExprVM::new();

    // INTEGER(10) / 0 -> NULL
    let program = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::Integer(0)),
        Op::Div,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert!(result.is_null());

    // INTEGER(10) % 0 -> NULL
    let program2 = Program::new(vec![
        Op::LoadConst(Value::Integer(10)),
        Op::LoadConst(Value::Integer(0)),
        Op::Mod,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert!(result2.is_null());
}

// =========================================================================
// AC-18: as_int64/as_float64 coercion for BIGINT and DECIMAL
// =========================================================================

#[test]
fn test_ac18_bigint_as_int64_coercion() {
    // AC-18: BIGINT -> i64 coercion (single limb only)
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT(42) -> as_int64 -> 42
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("42").unwrap())),
        Op::Cast(DataType::Integer),
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Integer(42));
}

#[test]
fn test_ac18_bigint_as_float64_coercion() {
    // AC-18: BIGINT -> f64 coercion via as_float64
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT(42) -> as_float64 -> 42.0
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("42").unwrap())),
        Op::Cast(DataType::Float),
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_ac18_decimal_as_int64_coercion() {
    // AC-18: DECIMAL -> i64 coercion (truncates fractional part)
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // DECIMAL(10.9) -> as_int64 -> 10 (truncated)
    let program = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.9").unwrap())),
        Op::Cast(DataType::Integer),
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Integer(10));

    // DECIMAL(10.9) - DECIMAL(1.0) = DECIMAL(9.9), then cast to Integer -> 9
    let program2 = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.9").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("1.0").unwrap())),
        Op::DecimalSub,
        Op::Cast(DataType::Integer),
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Integer(9));
}

#[test]
fn test_ac18_decimal_as_float64_coercion() {
    // AC-18: DECIMAL -> f64 coercion via as_float64
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // DECIMAL(10.5) -> as_float64 -> 10.5
    let program = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::Cast(DataType::Float),
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Float(10.5));
}

// =========================================================================
// AC-12: BTree index ordering for BIGINT and DECIMAL
// Tests that BigintCmp and DecimalCmp produce correct ordering results
// =========================================================================

#[test]
fn test_ac12_bigint_btree_ordering() {
    // AC-12: BIGINT comparison for BTree index ordering
    use crate::core::stoolap_parse_bigint;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // BIGINT(10) cmp BIGINT(5) -> 1 (greater)
    let program = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("5").unwrap())),
        Op::BigintCmp,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Integer(1));

    // BIGINT(5) cmp BIGINT(10) -> -1 (less)
    let program2 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("5").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::BigintCmp,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Integer(-1));

    // BIGINT(10) cmp BIGINT(10) -> 0 (equal)
    let program3 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("10").unwrap())),
        Op::BigintCmp,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert_eq!(result3, Value::Integer(0));

    // BIGINT(-10) cmp BIGINT(5) -> -1 (negative is less)
    let program4 = Program::new(vec![
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("-10").unwrap())),
        Op::LoadConst(Value::bigint(stoolap_parse_bigint("5").unwrap())),
        Op::BigintCmp,
        Op::Return,
    ]);
    let ctx4 = ExecuteContext::new(&row);
    let result4 = vm.execute(&program4, &ctx4).unwrap();
    assert_eq!(result4, Value::Integer(-1));
}

#[test]
fn test_ac12_decimal_btree_ordering() {
    // AC-12: DECIMAL comparison for BTree index ordering
    use crate::core::stoolap_parse_decimal;
    let row = Row::new();
    let mut vm = ExprVM::new();

    // DECIMAL(10.5) cmp DECIMAL(5.5) -> 1 (greater)
    let program = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("5.5").unwrap())),
        Op::DecimalCmp,
        Op::Return,
    ]);
    let ctx = ExecuteContext::new(&row);
    let result = vm.execute(&program, &ctx).unwrap();
    assert_eq!(result, Value::Integer(1));

    // DECIMAL(5.5) cmp DECIMAL(10.5) -> -1 (less)
    let program2 = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("5.5").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::DecimalCmp,
        Op::Return,
    ]);
    let ctx2 = ExecuteContext::new(&row);
    let result2 = vm.execute(&program2, &ctx2).unwrap();
    assert_eq!(result2, Value::Integer(-1));

    // DECIMAL(10.5) cmp DECIMAL(10.5) -> 0 (equal)
    let program3 = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("10.5").unwrap())),
        Op::DecimalCmp,
        Op::Return,
    ]);
    let ctx3 = ExecuteContext::new(&row);
    let result3 = vm.execute(&program3, &ctx3).unwrap();
    assert_eq!(result3, Value::Integer(0));

    // DECIMAL(-5.5) cmp DECIMAL(5.5) -> -1 (negative is less)
    let program4 = Program::new(vec![
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("-5.5").unwrap())),
        Op::LoadConst(Value::decimal(stoolap_parse_decimal("5.5").unwrap())),
        Op::DecimalCmp,
        Op::Return,
    ]);
    let ctx4 = ExecuteContext::new(&row);
    let result4 = vm.execute(&program4, &ctx4).unwrap();
    assert_eq!(result4, Value::Integer(-1));
}
