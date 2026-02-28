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

use crate::execution::gas::{GasMeter, GasPrice, GasPrices};

#[test]
fn test_gas_meter_charge() {
    let mut meter = GasMeter::new(1000);

    // Charge for reading a row (default 100 gas)
    assert!(meter.charge(GasPrice::ReadRow).is_ok());
    assert_eq!(meter.gas_used(), 100);
    assert_eq!(meter.remaining(), 900);

    // Charge for writing a row (default 1000 gas) - should fail because total would be 1100
    assert!(meter.charge(GasPrice::WriteRow).is_err());
    assert_eq!(meter.gas_used(), 100); // gas_used should not change on failed charge
    assert_eq!(meter.remaining(), 900);

    // Check that the meter is still under limit
    assert!(meter.check_gas_limit().is_ok());
}

#[test]
fn test_gas_meter_out_of_gas() {
    let mut meter = GasMeter::new(100);

    // Charge for reading a row (100 gas) - should succeed
    assert!(meter.charge(GasPrice::ReadRow).is_ok());
    assert_eq!(meter.gas_used(), 100);

    // Try to charge again - should fail due to out of gas
    let result = meter.charge(GasPrice::ReadRow);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e, crate::core::Error::OutOfGas { .. }));
    }
}

#[test]
fn test_gas_meter_check_limit() {
    let meter = GasMeter::new(500);

    // Should be under limit initially
    assert!(meter.check_gas_limit().is_ok());

    let mut meter = GasMeter::new(500);

    // Charge exactly to the limit (500)
    meter.charge(GasPrice::FullScan).unwrap(); // 500 gas

    // Should still be OK (at limit is allowed)
    assert!(meter.check_gas_limit().is_ok());

    // Trying to charge more should fail
    assert!(meter.charge(GasPrice::Compute).is_err());
}

#[test]
fn test_gas_meter_remaining() {
    let mut meter = GasMeter::new(10000);

    assert_eq!(meter.remaining(), 10000);

    meter.charge(GasPrice::IndexScan).unwrap(); // 50 gas
    assert_eq!(meter.remaining(), 9950);

    meter.charge(GasPrice::FullScan).unwrap(); // 500 gas
    assert_eq!(meter.remaining(), 9450);

    meter.charge(GasPrice::Compute).unwrap(); // 1 gas
    assert_eq!(meter.remaining(), 9449);
}

#[test]
fn test_gas_meter_with_custom_prices() {
    let prices = GasPrices {
        read_row: 50,
        write_row: 500,
        index_scan: 25,
        full_scan: 250,
        compute: 2,
    };

    let mut meter = GasMeter::with_prices(1000, prices);

    meter.charge(GasPrice::ReadRow).unwrap();
    assert_eq!(meter.gas_used(), 50);

    meter.charge(GasPrice::WriteRow).unwrap();
    assert_eq!(meter.gas_used(), 550);
}

#[test]
fn test_gas_prices_default() {
    let prices = GasPrices::default();
    assert_eq!(prices.read_row, 100);
    assert_eq!(prices.write_row, 1000);
    assert_eq!(prices.index_scan, 50);
    assert_eq!(prices.full_scan, 500);
    assert_eq!(prices.compute, 1);
}
