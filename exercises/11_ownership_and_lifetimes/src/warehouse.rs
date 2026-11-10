//! Part 3 — splitting borrows and moving out of `&mut`.

use std::marker::PhantomData;

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
    // engineer: stock, backorders, shipped, audit
}

impl Warehouse {
    pub fn new() -> Self {
        todo!()
    }

    /// Adds `qty` units of `sku` to stock. Saturates at `u32::MAX`.
    pub fn receive(&mut self, _sku: &str, _qty: u32) {
        todo!()
    }

    /// Units currently on hand for `sku` (0 if unknown).
    pub fn stock(&self, _sku: &str) -> u32 {
        todo!()
    }

    /// Queues an order at the back of the backorder queue.
    pub fn place_order(&mut self, _order: Order) -> Result<(), WarehouseError> {
        todo!()
    }

    /// Ships every queued order that current stock can cover, in FIFO order,
    /// skipping blocked orders (they keep their relative order). Returns how
    /// many orders shipped.
    pub fn fulfil(&mut self) -> usize {
        todo!()
    }

    /// Removes the first queued order with `order_id`.
    pub fn cancel(&mut self, _order_id: u64) -> Option<Order> {
        todo!()
    }

    /// Borrowed view of the queued orders, FIFO.
    pub fn backorders(&self) -> &[Order] {
        todo!()
    }

    /// Moves all shipments out, leaving the list empty.
    pub fn take_shipments(&mut self) -> Vec<Shipment> {
        todo!()
    }

    /// Moves the audit log out, leaving it empty.
    pub fn take_audit(&mut self) -> Vec<String> {
        todo!()
    }

    /// Replaces the whole backorder queue, returning the previous one.
    pub fn replace_backorders(&mut self, _orders: Vec<Order>) -> Vec<Order> {
        todo!()
    }
}

/// Mutable references to two distinct elements. `None` if `i == j` or either index is out of bounds.
pub fn split_pair_mut<T>(_items: &mut [T], _i: usize, _j: usize) -> Option<(&mut T, &mut T)> {
    todo!()
}

/// A live value plus a staging buffer, swapped in by [`Staged::publish`].
#[derive(Debug)]
pub struct Staged<T> {
    // engineer: live and staged values
    _marker: PhantomData<T>,
}

impl<T: Clone> Staged<T> {
    /// Both buffers start as copies of `value`.
    pub fn new(_value: T) -> Self {
        todo!()
    }

    /// The published value.
    pub fn live(&self) -> &T {
        todo!()
    }

    /// The staging buffer. Writes here are invisible until [`Staged::publish`].
    pub fn staged_mut(&mut self) -> &mut T {
        todo!()
    }

    /// Makes the staged value live. The staging buffer then holds the previous live value.
    pub fn publish(&mut self) {
        todo!()
    }
}
