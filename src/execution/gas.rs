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

//! Gas metering for transaction execution
//!
//! This module provides gas metering functionality to track and limit
//! the computational resources used during transaction execution.

use crate::core::{Error, Result};

/// Gas price types for different operations
///
/// Each variant represents a type of operation with an associated gas cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasPrice {
    /// Gas cost for reading a row
    ReadRow,

    /// Gas cost for writing a row
    WriteRow,

    /// Gas cost for scanning an index
    IndexScan,

    /// Gas cost for a full table scan
    FullScan,

    /// Gas cost for general computation
    Compute,
}

/// Gas prices for different operation types
///
/// Defines the gas cost associated with each type of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GasPrices {
    /// Gas cost for reading a row
    pub read_row: u64,

    /// Gas cost for writing a row
    pub write_row: u64,

    /// Gas cost for scanning an index
    pub index_scan: u64,

    /// Gas cost for a full table scan
    pub full_scan: u64,

    /// Gas cost for general computation
    pub compute: u64,
}

impl Default for GasPrices {
    fn default() -> Self {
        Self {
            read_row: 100,
            write_row: 1000,
            index_scan: 50,
            full_scan: 500,
            compute: 1,
        }
    }
}

impl GasPrices {
    /// Get the gas cost for a given price type
    fn get_cost(&self, price: GasPrice) -> u64 {
        match price {
            GasPrice::ReadRow => self.read_row,
            GasPrice::WriteRow => self.write_row,
            GasPrice::IndexScan => self.index_scan,
            GasPrice::FullScan => self.full_scan,
            GasPrice::Compute => self.compute,
        }
    }
}

/// Gas meter for tracking and limiting gas consumption
///
/// The gas meter tracks the amount of gas used during transaction execution
/// and ensures that operations do not exceed the specified gas limit.
#[derive(Debug, Clone)]
pub struct GasMeter {
    /// Amount of gas used so far
    gas_used: u64,

    /// Maximum amount of gas allowed
    gas_limit: u64,

    /// Gas prices for different operations
    prices: GasPrices,
}

impl GasMeter {
    /// Create a new gas meter with default gas prices
    ///
    /// # Arguments
    ///
    /// * `gas_limit` - Maximum amount of gas allowed
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::execution::gas::GasMeter;
    ///
    /// let meter = GasMeter::new(1000);
    /// ```
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_used: 0,
            gas_limit,
            prices: GasPrices::default(),
        }
    }

    /// Create a new gas meter with custom gas prices
    ///
    /// # Arguments
    ///
    /// * `gas_limit` - Maximum amount of gas allowed
    /// * `prices` - Custom gas prices for different operations
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::execution::gas::{GasMeter, GasPrices};
    ///
    /// let prices = GasPrices {
    ///     read_row: 50,
    ///     write_row: 500,
    ///     ..Default::default()
    /// };
    /// let meter = GasMeter::with_prices(1000, prices);
    /// ```
    pub fn with_prices(gas_limit: u64, prices: GasPrices) -> Self {
        Self {
            gas_used: 0,
            gas_limit,
            prices,
        }
    }

    /// Charge gas for an operation
    ///
    /// Deducts the gas cost for the specified operation from the remaining gas.
    /// Returns an error if the gas limit would be exceeded.
    ///
    /// # Arguments
    ///
    /// * `price` - The type of operation to charge for
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the gas was charged successfully, or `Err(Error::OutOfGas)`
    /// if the gas limit would be exceeded.
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::execution::gas::{GasMeter, GasPrice};
    ///
    /// let mut meter = GasMeter::new(1000);
    /// assert!(meter.charge(GasPrice::ReadRow).is_ok());
    /// ```
    pub fn charge(&mut self, price: GasPrice) -> Result<()> {
        let cost = self.prices.get_cost(price);

        // Check for overflow first
        let new_gas_used = self
            .gas_used
            .checked_add(cost)
            .ok_or(Error::GasOverflow)?;

        // Check if this charge would exceed the limit
        if new_gas_used > self.gas_limit {
            return Err(Error::OutOfGas {
                used: new_gas_used,
                limit: self.gas_limit,
            });
        }

        self.gas_used = new_gas_used;
        Ok(())
    }

    /// Check if the gas limit has been exceeded
    ///
    /// Returns an error if the gas used exceeds the gas limit.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if under the limit, or `Err(Error::OutOfGas)` if exceeded.
    pub fn check_gas_limit(&self) -> Result<()> {
        if self.gas_used > self.gas_limit {
            return Err(Error::OutOfGas {
                used: self.gas_used,
                limit: self.gas_limit,
            });
        }
        Ok(())
    }

    /// Get the remaining gas
    ///
    /// Returns the amount of gas remaining before hitting the limit.
    /// This value can be negative if the limit has been exceeded.
    ///
    /// # Returns
    ///
    /// The remaining gas as a signed 64-bit integer
    pub fn remaining(&self) -> i64 {
        self.gas_limit as i64 - self.gas_used as i64
    }

    /// Get the total gas used so far
    ///
    /// # Returns
    ///
    /// The total gas used
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    /// Get the gas limit
    ///
    /// # Returns
    ///
    /// The gas limit
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_prices_default() {
        let prices = GasPrices::default();
        assert_eq!(prices.read_row, 100);
        assert_eq!(prices.write_row, 1000);
        assert_eq!(prices.index_scan, 50);
        assert_eq!(prices.full_scan, 500);
        assert_eq!(prices.compute, 1);
    }

    #[test]
    fn test_gas_meter_new() {
        let meter = GasMeter::new(1000);
        assert_eq!(meter.gas_used(), 0);
        assert_eq!(meter.gas_limit(), 1000);
        assert_eq!(meter.remaining(), 1000);
    }

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
            assert!(matches!(e, Error::OutOfGas { .. }));
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
    fn test_gas_overflow() {
        let mut meter = GasMeter::new(u64::MAX);
        meter.gas_used = u64::MAX - 10;

        // Try to charge more than would fit in u64
        let result = meter.charge(GasPrice::ReadRow);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, Error::GasOverflow));
        }
    }
}
