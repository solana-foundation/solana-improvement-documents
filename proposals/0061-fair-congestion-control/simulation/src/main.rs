#![feature(adt_const_params)]
#![allow(incomplete_features)]

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
struct BaseFeeTracker<const POLICY: Policy> {
    clock: u64,
    reset_counter: u64,
    is_congested: bool,
    fee_markets: BTreeMap<Addr, LocalFeeMarket>,
    active_txs: BTreeSet<TxId>,
    nonconflicting_group_count: u64,
    recent_addrs: VecDeque<Vec<Addr>>,
    rewarded_cu: u64,
    total_supplied_fee: u64,
}

const MINIMUM_BASE_FEE_RATE: u64 = 5000;
const MAXIMUM_THREAD_COUNT: usize = 5;
const CU_TO_POWER: f64 = 50_000.0;
const INITIAL_RESERVED_FEE: u64 = 0;

#[derive(Debug, PartialEq)]
enum MeasureError {
    AlreadyMeasuring,
    AlreadyActive,
    NoAddress,
    NotMeasured,
    TooManyActiveThreadCount,
    InsufficientSuppliedFee(u64, u64),
}
use MeasureError::*;

#[derive(PartialEq, Eq)]
struct Policy {
    congestion_threshold: usize,
    recent_tx_count: usize,
}

impl Policy {
    const fn new() -> Self {
        Self {
            congestion_threshold: 0,
            recent_tx_count: 5,
        }
    }

    const fn congestion_threshold(mut self, u: usize) -> Self {
        self.congestion_threshold = u;
        self
    }
}

impl<const POLICY: Policy> BaseFeeTracker<POLICY> {
    fn start_measuring(&mut self, tx: &Tx) -> Result<(), MeasureError> {
        let updated_active_tx_count = self.active_txs.len() + 1;

        if updated_active_tx_count > MAXIMUM_THREAD_COUNT {
            return Err(TooManyActiveThreadCount);
        }
        if tx.addrs.is_empty() {
            return Err(NoAddress);
        }

        let (is_congested, reset_counter) =
            match updated_active_tx_count.cmp(&POLICY.congestion_threshold) {
                Less if self.is_congested => (false, self.reset_counter + 1),
                Equal | Greater if !self.is_congested => (true, self.reset_counter),
                _ => (self.is_congested, self.reset_counter),
            };

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

                if is_congested && reset_counter == market.reset_counter {
                    let mut required_fee_rate = market.required_fee_rate;
                    required_fee_rate = self.cool_down(required_fee_rate, cool_down_duration);
                    required_fee_rate = required_fee_rate.max(MINIMUM_BASE_FEE_RATE);
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
        if !self.active_txs.insert(tx.id) {
            return Err(AlreadyMeasuring);
        }
        self.is_congested = is_congested;
        self.reset_counter = reset_counter;
        if is_new_group {
            self.nonconflicting_group_count += 1;
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
        if self.recent_addrs.len() > POLICY.recent_tx_count {
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
                self.nonconflicting_group_count -= 1;
            }
        }

        Ok(())
    }

    fn stop_measuring(&mut self, tx: &Tx, result: Result<(), u64>) -> Result<(), MeasureError> {
        if !self.active_txs.remove(&tx.id) {
            return Err(NotMeasured);
        }
        self.clock += tx.requested_cu;
        for addr in &tx.addrs {
            let market = self.fee_markets.get_mut(addr).unwrap();
            market.clock = self.clock;
            market.is_active = false;
        }
        self.rewarded_cu += tx.addrs.len() as u64
            * match result {
                Ok(()) => tx.requested_cu,
                Err(actual_cu) => actual_cu / 2,
            };

        Ok(())
    }

    fn heat_up(&self, fee_rate: u64, cu: u64) -> u64 {
        let factor = 2_f64.powf(cu as f64 / CU_TO_POWER);
        (fee_rate as f64 * factor) as u64
    }

    fn cool_down(&self, fee_rate: u64, cu: u64) -> u64 {
        let factor = 2_f64.powf(cu as f64 / self.nonconflicting_group_count.max(1) as f64 / CU_TO_POWER);
        (fee_rate as f64 / factor) as u64
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
    fn tracker_default() {
        let tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        assert_eq!(tracker.nonconflicting_group_count, 0);
        assert_eq!(tracker.burnt_fee(), 0);
        assert_eq!(tracker.collected_fee(), 0);
        assert_eq!(tracker.fee_markets.is_empty(), true);
    }

    #[test]
    fn exponential_heat_up() {
        let tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let cu = CU_TO_POWER as u64;
        assert_eq!(tracker.heat_up(5000, cu * 0), 5000 * 1);
        assert_eq!(tracker.heat_up(5000, cu * 1), 5000 * 2);
        assert_eq!(tracker.heat_up(5000, cu * 2), 5000 * 4);
        assert_eq!(tracker.heat_up(5000, cu * 3), 5000 * 8);
    }

    #[test]
    fn exponential_normal_cool_down() {
        let tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let cu = CU_TO_POWER as u64;
        assert_eq!(tracker.cool_down(5000 * 8, cu * 0), 5000 * 8);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 1), 5000 * 4);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 2), 5000 * 2);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 3), 5000 * 1);
    }

    #[test]
    fn exponential_slow_cool_down() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        tracker.nonconflicting_group_count = 5;
        let cu = CU_TO_POWER as u64;
        assert_eq!(tracker.cool_down(5000 * 8, cu * 0 * 5), 5000 * 8);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 1 * 5), 5000 * 4);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 2 * 5), 5000 * 2);
        assert_eq!(tracker.cool_down(5000 * 8, cu * 3 * 5), 5000 * 1);
    }

    #[test]
    fn tracker_no_congestion() {
        let mut tracker =
            BaseFeeTracker::<{ Policy::new().congestion_threshold(usize::MAX) }>::default();
        let tx = Tx::new(3, 200, 5000, vec![Addr(7)]);
        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.is_congested, false);
        assert_eq!(tracker.stop_measuring(&tx, Ok(())), Ok(()));

        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx, Ok(())), Ok(()));
    }

    #[test]
    fn tracker_congestion() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let tx = Tx::new(3, 200, 1002600/200, vec![Addr(7)]);
        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.is_congested, true);
        assert_eq!(tracker.stop_measuring(&tx, Ok(())), Ok(()));

        assert_eq!(tracker.start_measuring(&tx), Err(InsufficientSuppliedFee(1002600, 1005200)));
        let tx = Tx::new(3, 200, 1005200/200, vec![Addr(7)]);
        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx, Ok(())), Ok(()));
    }

    #[test]
    fn tracker_locality() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let tx1 = Tx::new(3, 200, 1002600/200, vec![Addr(7)]);
        let tx2 = Tx::new(4, 200, 1002600/200, vec![Addr(8)]);
        assert_eq!(tracker.start_measuring(&tx1), Ok(()));
        assert_eq!(tracker.is_congested, true);
        assert_eq!(tracker.stop_measuring(&tx1, Ok(())), Ok(()));

        assert_eq!(tracker.start_measuring(&tx1), Err(InsufficientSuppliedFee(1002600, 1005200)));

        assert_eq!(tracker.start_measuring(&tx2), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx2, Ok(())), Ok(()));
    }

    #[test]
    fn tracker_cool_down() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let tx1 = Tx::new(3, 200, 1002600/200, vec![Addr(7)]);
        let tx2 = Tx::new(4, 200, 1002600/200, vec![Addr(8)]);
        assert_eq!(tracker.start_measuring(&tx1), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx1, Ok(())), Ok(()));

        assert_eq!(tracker.start_measuring(&tx2), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx2, Ok(())), Ok(()));
        assert_eq!(tracker.nonconflicting_group_count, 1);

        assert_eq!(tracker.start_measuring(&tx1), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx1, Ok(())), Ok(()));

        assert_eq!(tracker.start_measuring(&tx2), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx2, Ok(())), Ok(()));
    }

    #[test]
    fn tracker_insufficient_fee() {
        let mut tracker =
            BaseFeeTracker::<{ Policy::new().congestion_threshold(usize::MAX) }>::default();
        let tx = Tx::new(3, 200, 4999, vec![Addr(7)]);
        assert_eq!(
            tracker.start_measuring(&tx),
            Err(InsufficientSuppliedFee(999800, 1000000))
        );
    }

    #[test]
    fn tracker_burn_and_collect_with_success_tx() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let cu = 200;
        let tx = Tx::new(3, cu, 1002600 / cu, vec![Addr(7)]);
        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx, Ok(())), Ok(()));
        assert_eq!(tracker.burnt_fee(), 2600);
        assert_eq!(tracker.collected_fee(), cu * MINIMUM_BASE_FEE_RATE);
    }

    #[test]
    fn tracker_burn_and_collect_with_fail_tx() {
        let mut tracker = BaseFeeTracker::<{ Policy::new() }>::default();
        let cu = 200;
        let actual_cu = 100;
        let tx = Tx::new(3, cu, 1002600 / cu, vec![Addr(7)]);
        assert_eq!(tracker.start_measuring(&tx), Ok(()));
        assert_eq!(tracker.stop_measuring(&tx, Err(actual_cu)), Ok(()));
        assert_eq!(tracker.burnt_fee(), 1000000 - 1000000/4 + 2600);
        assert_eq!(tracker.collected_fee(), (actual_cu / 2) * MINIMUM_BASE_FEE_RATE);
    }
}
