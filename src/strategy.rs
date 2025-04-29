use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type Price = Decimal;
pub type Size = Decimal;
pub type Profit = Decimal;

pub struct Id(pub usize);

pub const SELL_FEE: Decimal = dec!(0.15);

pub struct Market {
    pub id: Id,
    pub orderbook: Orderbook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub price: Price,
    pub size: Size,
}

pub struct Orderbook {
    asks: BTreeMap<Price, Level>,
    bids: BTreeMap<Price, Level>,
}

impl Orderbook {
    pub fn new<Bids, Asks>(bids: Bids, asks: Asks) -> Self
    where
        Asks: IntoIterator<Item = Level>,
        Bids: IntoIterator<Item = Level>,
    {
        Self {
            asks: asks.into_iter().map(|level| (level.price, level)).collect(),
            bids: bids.into_iter().map(|level| (level.price, level)).collect(),
        }
    }

    pub fn bids(&self) -> impl Iterator<Item = &Level> {
        self.bids.values().rev()
    }

    pub fn asks(&self) -> impl Iterator<Item = &Level> {
        self.asks.values()
    }
}

/// Determines profit from spread
pub fn calc_profit_from_spread(ob: &Orderbook) -> Option<Profit> {
    let best_ask = ob.asks().next()?;
    let best_bid = ob.bids().next()?;
    let gross_profit = best_ask.price - best_bid.price;
    Some(gross_profit - (best_ask.price * SELL_FEE))
}

pub struct ProfitResult<'a> {
    inner: BTreeMap<Profit, &'a Market>,
}

impl<'a> ProfitResult<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (&Profit, &&Market)> {
        self.inner.iter().rev()
    }

    pub fn best(&self) -> Option<(&Profit, &&Market)> {
        self.iter().next()
    }
}

pub fn find_profit<'a, Markets>(obs: Markets) -> ProfitResult<'a>
where
    Markets: IntoIterator<Item = &'a Market>,
{
    ProfitResult {
        inner: obs
            .into_iter()
            .filter_map(|market| {
                let profit = calc_profit_from_spread(&market.orderbook)?;
                Some((profit, market))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_profit_works() {
        let ob = Orderbook::new(
            [
                Level {
                    price: dec!(1),
                    size: dec!(1),
                },
                Level {
                    price: dec!(2),
                    size: dec!(1),
                },
            ],
            [
                Level {
                    price: dec!(3),
                    size: dec!(1),
                },
                Level {
                    price: dec!(4),
                    size: dec!(1),
                },
            ],
        );

        let profit = calc_profit_from_spread(&ob).unwrap();
        assert_eq!(profit, dec!(1) - (dec!(3) * SELL_FEE));
    }

    #[test]
    fn find_best_profit() {
        let obs = [
            Market {
                orderbook: Orderbook::new(
                    [Level {
                        price: dec!(2),
                        size: dec!(1),
                    }],
                    [Level {
                        price: dec!(3),
                        size: dec!(1),
                    }],
                ),
                id: Id(1),
            },
            Market {
                orderbook: Orderbook::new(
                    [Level {
                        price: dec!(2),
                        size: dec!(1),
                    }],
                    [Level {
                        price: dec!(4),
                        size: dec!(1),
                    }],
                ),
                id: Id(2),
            },
            Market {
                orderbook: Orderbook::new(
                    [Level {
                        price: dec!(2),
                        size: dec!(1),
                    }],
                    [Level {
                        price: dec!(5),
                        size: dec!(1),
                    }],
                ),
                id: Id(3),
            },
        ];
        let result = find_profit(&obs);
        let best = result.best().unwrap();
        assert_eq!(*best.0, dec!(3) - (dec!(5) * SELL_FEE));
        assert_eq!(result.iter().count(), 3);
    }
}
