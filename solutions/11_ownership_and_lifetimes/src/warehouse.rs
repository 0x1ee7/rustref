//! Part 3 — splitting borrows and moving out of `&mut`.

use std::collections::BTreeMap;
use std::mem;

/// A customer order for `qty` units of `sku`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    pub sku: String,
    pub qty: u32,
}

/// An order that was fully shipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shipment {
    pub order_id: u64,
    pub sku: String,
    pub qty: u32,
}

/// Errors from [`Warehouse::place_order`].
#[derive(Debug, PartialEq, Eq)]
pub enum WarehouseError {
    /// Orders must ask for at least one unit.
    ZeroQuantity { order_id: u64 },
}

/// Stock levels, a FIFO backorder queue, shipments and an audit log.
#[derive(Debug, Default)]
pub struct Warehouse {
    stock: BTreeMap<String, u32>,
    backorders: Vec<Order>,
    shipments: Vec<Shipment>,
    audit: Vec<String>,
}

impl Warehouse {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `qty` units of `sku` to stock. Saturates at `u32::MAX`.
    pub fn receive(&mut self, sku: &str, qty: u32) {
        match self.stock.get_mut(sku) {
            Some(on_hand) => *on_hand = on_hand.saturating_add(qty),
            None => {
                self.stock.insert(sku.to_string(), qty);
            }
        }
    }

    /// Units currently on hand for `sku` (0 if unknown).
    pub fn stock(&self, sku: &str) -> u32 {
        self.stock.get(sku).copied().unwrap_or(0)
    }

    /// Queues an order at the back of the backorder queue.
    pub fn place_order(&mut self, order: Order) -> Result<(), WarehouseError> {
        if order.qty == 0 {
            return Err(WarehouseError::ZeroQuantity { order_id: order.id });
        }
        self.backorders.push(order);
        Ok(())
    }

    /// Ships every queued order that current stock can cover, in FIFO order,
    /// skipping blocked orders (they keep their relative order). Returns how
    /// many orders shipped.
    pub fn fulfil(&mut self) -> usize {
        // Destructure so the stock map, the queue and the outputs can be
        // borrowed independently in the same loop.
        let Warehouse { stock, backorders, shipments, audit } = self;
        let mut shipped = 0;
        let mut blocked = Vec::new();
        for order in mem::take(backorders) {
            match stock.get_mut(&order.sku) {
                Some(on_hand) if *on_hand >= order.qty => {
                    *on_hand -= order.qty;
                    audit.push(format!("shipped order {}", order.id));
                    shipments.push(Shipment {
                        order_id: order.id,
                        sku: order.sku,
                        qty: order.qty,
                    });
                    shipped += 1;
                }
                _ => blocked.push(order),
            }
        }
        *backorders = blocked;
        shipped
    }

    /// Removes the first queued order with `order_id`.
    pub fn cancel(&mut self, order_id: u64) -> Option<Order> {
        let pos = self.backorders.iter().position(|o| o.id == order_id)?;
        Some(self.backorders.remove(pos))
    }

    /// Borrowed view of the queued orders, FIFO.
    pub fn backorders(&self) -> &[Order] {
        &self.backorders
    }

    /// Moves all shipments out, leaving the list empty.
    pub fn take_shipments(&mut self) -> Vec<Shipment> {
        mem::take(&mut self.shipments)
    }

    /// Moves the audit log out, leaving it empty.
    pub fn take_audit(&mut self) -> Vec<String> {
        mem::take(&mut self.audit)
    }

    /// Replaces the whole backorder queue, returning the previous one.
    pub fn replace_backorders(&mut self, orders: Vec<Order>) -> Vec<Order> {
        mem::replace(&mut self.backorders, orders)
    }
}

/// Mutable references to two distinct elements. `None` if `i == j` or either index is out of bounds.
pub fn split_pair_mut<T>(items: &mut [T], i: usize, j: usize) -> Option<(&mut T, &mut T)> {
    if i == j || i >= items.len() || j >= items.len() {
        return None;
    }
    let (lo, hi) = if i < j { (i, j) } else { (j, i) };
    let (left, right) = items.split_at_mut(hi);
    let low = &mut left[lo];
    let high = &mut right[0];
    if i < j {
        Some((low, high))
    } else {
        Some((high, low))
    }
}

/// A live value plus a staging buffer, swapped in by [`Staged::publish`].
#[derive(Debug)]
pub struct Staged<T> {
    live: T,
    staged: T,
}

impl<T: Clone> Staged<T> {
    /// Both buffers start as copies of `value`.
    pub fn new(value: T) -> Self {
        Staged {
            staged: value.clone(),
            live: value,
        }
    }

    /// The published value.
    pub fn live(&self) -> &T {
        &self.live
    }

    /// The staging buffer. Writes here are invisible until [`Staged::publish`].
    pub fn staged_mut(&mut self) -> &mut T {
        &mut self.staged
    }

    /// Makes the staged value live. The staging buffer then holds the previous live value.
    pub fn publish(&mut self) {
        mem::swap(&mut self.live, &mut self.staged);
    }
}
