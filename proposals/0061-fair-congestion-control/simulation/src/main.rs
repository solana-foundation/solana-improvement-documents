use std::cmp::Ordering::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Clone, Copy, Default)]
struct Addr(u8);

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Clone, Copy, Default)]
struct TxId(u8);

#[derive(Debug, Clone)]
struct Tx {
    id: TxId,
    requested_cu: u64,
    supplied_fee_rate: u64,
    addrs: Vec<Addr>,
}

// multiply fee by requested_cu?

impl Tx {
    fn new(id: u8, requested_cu: u64, supplied_fee_rate: u64, addrs: Vec<Addr>) -> Self {
        Self {
            id: TxId(id),
            requested_cu,
            supplied_fee_rate,
            addrs,
        }
    }
}

#[derive(Debug)]
struct LocalFeeMarket {
    required_fee_rate: u64,
    reserved_fee: u64, // accrued like 106% per 1 Mcu??? depending on active tx count?
    clock: u64,
    reset_counter: u64,
    freq: u64,
    is_active: bool,
}

impl LocalFeeMarket {
    fn new(reset_counter: u64) -> Self {
        Self {
            required_fee_rate: MINIMUM_BASE_FEE_RATE,
            reserved_fee: INITIAL_RESERVED_FEE,
            clock: 0,
            reset_counter,
            freq: 0,
            is_active: false,
        }
    }
}

#[derive(Default, Debug)]
struct BaseFeeTracker<const CONGESTION_THRESHOLD: usize> {
    clock: u64,
    reset_counter: u64,
    is_congested: bool,
    fee_markets: BTreeMap<Addr, LocalFeeMarket>,
    active_txs: BTreeSet<TxId>,
    tx_group_count: u64,
    recent_addrs: VecDeque<Vec<Addr>>,
    rewarded_cu: u64,
    total_supplied_fee: u64,
}

const MINIMUM_BASE_FEE_RATE: u64 = 5000;
const MAXIMUM_THREAD_COUNT: usize = 5;
const RECENT_TX_COUNT: usize = 5;
const CU_TO_POWER: f64 = 50_000.0;
const INITIAL_RESERVED_FEE: u64 = 0;

#[derive(Debug)]
enum MeasureError {
    AlreadyMeasuring,
    AlreadyActive,
    NotMeasured,
    TooManyActiveThreadCount,
    InsufficientSuppliedFee(u64, u64),
}
use MeasureError::*;

impl<const CONGESTION_THRESHOLD: usize> BaseFeeTracker<CONGESTION_THRESHOLD> {
    fn start_measuring(&mut self, tx: &Tx) -> Result<(), MeasureError> {
        if !self.active_txs.insert(tx.id) {
            return Err(AlreadyMeasuring);
        }
        if self.active_txs.len() > MAXIMUM_THREAD_COUNT {
            return Err(TooManyActiveThreadCount);
        }

        match self.active_txs.len().cmp(&CONGESTION_THRESHOLD) {
            Less => {
                if self.is_congested {
                    self.is_congested = false;
                    self.reset_counter += 1;
                }
            }
            Equal | Greater => {
                if !self.is_congested {
                    self.is_congested = true;
                }
            }
        }

        let is_new_group = tx.addrs.iter().all(|addr| {
            self.fee_markets
                .entry(*addr)
                .or_insert_with(|| LocalFeeMarket::new(self.reset_counter))
                .freq
                == 0
        });
        tx.addrs
            .iter()
            .map(|addr| {
                if self.fee_markets.get(addr).unwrap().is_active {
                    Err(AlreadyActive)
                } else {
                    Ok(())
                }
            })
            .collect::<Result<_, _>>()?;

        let heat_up_duration = tx.requested_cu;
        let updated_fees = tx
            .addrs
            .iter()
            .map(|addr| {
                let market = self.fee_markets.get(addr).unwrap();
                let cool_down_duration = self.clock - market.clock;
                let reserved_fee = (market.reserved_fee as f64
                    * 1.06_f64.powf(cool_down_duration as f64 / 1_000_000_f64))
                    as u64;

                if self.is_congested && self.reset_counter == market.reset_counter {
                    let mut required_fee_rate = market.required_fee_rate;
                    required_fee_rate = self.cool_down(required_fee_rate, cool_down_duration);
                    required_fee_rate = self.heat_up(required_fee_rate, heat_up_duration);

                    (required_fee_rate, reserved_fee)
                } else {
                    (MINIMUM_BASE_FEE_RATE, reserved_fee)
                }
            })
            .collect::<Vec<_>>();
        let minimum_supplied_fee = updated_fees
            .iter()
            .map(|&(required_fee_rate, reserved_fee)| {
                let required_fee = required_fee_rate * tx.requested_cu;
                if self.is_congested {
                    required_fee.saturating_sub(reserved_fee)
                } else {
                    required_fee
                }
            })
            .sum::<u64>();
        let supplied_fee = tx.supplied_fee_rate * tx.requested_cu;
        if supplied_fee < minimum_supplied_fee {
            return Err(InsufficientSuppliedFee(supplied_fee, minimum_supplied_fee));
        }
        if is_new_group {
            self.tx_group_count += 1;
        }
        self.total_supplied_fee += supplied_fee;
        let total_excess_fee = (supplied_fee - minimum_supplied_fee) as f64;

        let total_required_fee = updated_fees
            .iter()
            .map(|(required_fee_rate, _)| required_fee_rate * tx.requested_cu)
            .sum::<u64>() as f64;
        for (addr, &(required_fee_rate, mut reserved_fee)) in
            tx.addrs.iter().zip(updated_fees.iter())
        {
            let required_fee = required_fee_rate * tx.requested_cu;
            if self.is_congested {
                reserved_fee = reserved_fee.saturating_sub(required_fee);
            }
            reserved_fee +=
                (total_excess_fee * ((required_fee as f64) / total_required_fee)) as u64;
            reserved_fee = (reserved_fee as f64
                * 1.06_f64.powf(heat_up_duration as f64 / 1_000_000_f64))
                as u64;

            let market = self.fee_markets.get_mut(addr).unwrap();
            market.required_fee_rate = required_fee_rate;
            market.reserved_fee = reserved_fee;
            market.reset_counter = self.reset_counter;
            market.freq += 1;
            market.is_active = true;
        }

        self.recent_addrs.push_back(tx.addrs.clone());
        if self.recent_addrs.len() > RECENT_TX_COUNT {
            let was_new_group = self
                .recent_addrs
                .pop_front()
                .unwrap()
                .iter()
                .all(|expired_addr| {
                    let market = self.fee_markets.get_mut(expired_addr).unwrap();
                    market.freq -= 1;
                    market.freq == 0
                });
            if was_new_group {
                self.tx_group_count -= 1;
            }
        }

        Ok(())
    }

    fn stop_measuring(&mut self, tx: &Tx) -> Result<(), MeasureError> {
        let result: Result<_, u64> = Ok(());
        if !self.active_txs.remove(&tx.id) {
            return Err(NotMeasured);
        }
        self.clock += tx.requested_cu;
        for addr in &tx.addrs {
            let market = self.fee_markets.get_mut(addr).unwrap();
            market.clock = self.clock;
            market.is_active = false;
        }
        self.rewarded_cu += match result {
            Ok(()) => tx.requested_cu,
            Err(cu) => cu / 2,
        };

        Ok(())
    }

    fn heat_up(&self, current_required_base_fee: u64, cu: u64) -> u64 {
        let factor = 2_f64.powf(cu as f64 / CU_TO_POWER);
        (current_required_base_fee as f64 * factor) as u64
    }

    fn cool_down(&self, current_required_base_fee: u64, cu: u64) -> u64 {
        let inverse_factor = 0.5_f64.powf(cu as f64 / self.tx_group_count as f64 / CU_TO_POWER);
        ((current_required_base_fee as f64 * inverse_factor) as u64).max(MINIMUM_BASE_FEE_RATE)
    }

    #[allow(dead_code)]
    fn collected_fee(&self) -> u64 {
        self.rewarded_cu * MINIMUM_BASE_FEE_RATE
    }

    #[allow(dead_code)]
    fn burnt_fee(&self) -> u64 {
        self.total_supplied_fee - self.collected_fee()
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_base_fee_rate() {
        let tracker = BaseFeeTracker::<0>::default();
    }
}
