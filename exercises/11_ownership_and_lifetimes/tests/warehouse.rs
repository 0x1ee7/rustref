use ex11_ownership_and_lifetimes::warehouse::{
    split_pair_mut, Order, Shipment, Staged, Warehouse, WarehouseError,
};

fn order(id: u64, sku: &str, qty: u32) -> Order {
    Order { id, sku: sku.to_string(), qty }
}

fn shipped_ids(shipments: &[Shipment]) -> Vec<u64> {
    shipments.iter().map(|s| s.order_id).collect()
}

fn queued_ids(w: &Warehouse) -> Vec<u64> {
    w.backorders().iter().map(|o| o.id).collect()
}

#[test]
fn part3_fulfil_ships_in_fifo_order_while_stock_lasts() {
    let mut w = Warehouse::new();
    w.receive("bolt", 5);
    for id in 1..=3 {
        w.place_order(order(id, "bolt", 2)).unwrap();
    }
    assert_eq!(w.fulfil(), 2);
    assert_eq!(shipped_ids(&w.take_shipments()), vec![1, 2]);
    assert_eq!(queued_ids(&w), vec![3]);
    assert_eq!(w.stock("bolt"), 1);
}

#[test]
fn part3_blocked_orders_keep_order_and_later_orders_can_ship() {
    let mut w = Warehouse::new();
    w.receive("bolt", 3);
    w.place_order(order(1, "bolt", 10)).unwrap();
    w.place_order(order(2, "bolt", 2)).unwrap();
    w.place_order(order(3, "bolt", 1)).unwrap();

    assert_eq!(w.fulfil(), 2, "orders 2 and 3 fit in stock; order 1 does not");
    assert_eq!(shipped_ids(&w.take_shipments()), vec![2, 3]);
    assert_eq!(queued_ids(&w), vec![1]);

    w.receive("bolt", 7);
    assert_eq!(w.fulfil(), 0, "still 7 units short of order 1");
    w.receive("bolt", 3);
    assert_eq!(w.fulfil(), 1);
    assert_eq!(shipped_ids(&w.take_shipments()), vec![1]);
    assert!(w.backorders().is_empty());
}

#[test]
fn part3_fulfil_without_stock_ships_nothing() {
    let mut w = Warehouse::new();
    w.place_order(order(1, "nut", 1)).unwrap();
    assert_eq!(w.fulfil(), 0);
    assert_eq!(queued_ids(&w), vec![1]);
    assert!(w.take_shipments().is_empty());
}

#[test]
fn part3_each_shipment_is_audited() {
    let mut w = Warehouse::new();
    w.receive("nut", 1);
    w.place_order(order(7, "nut", 1)).unwrap();
    w.fulfil();
    assert_eq!(w.take_audit(), vec!["shipped order 7".to_string()]);
}

#[test]
fn part3_take_shipments_moves_out_and_leaves_empty() {
    let mut w = Warehouse::new();
    w.receive("x", 2);
    w.place_order(order(1, "x", 1)).unwrap();
    w.place_order(order(2, "x", 1)).unwrap();
    w.fulfil();
    assert_eq!(shipped_ids(&w.take_shipments()), vec![1, 2]);
    assert!(w.take_shipments().is_empty(), "second take must be empty");
}

#[test]
fn part3_take_audit_drains_the_log() {
    let mut w = Warehouse::new();
    w.receive("x", 1);
    w.place_order(order(1, "x", 1)).unwrap();
    w.fulfil();
    assert_eq!(w.take_audit().len(), 1);
    assert!(w.take_audit().is_empty());
}

#[test]
fn part3_place_order_rejects_zero_quantity() {
    let mut w = Warehouse::new();
    assert_eq!(
        w.place_order(order(4, "x", 0)),
        Err(WarehouseError::ZeroQuantity { order_id: 4 })
    );
    assert!(w.backorders().is_empty(), "rejected orders are not queued");
}

#[test]
fn part3_cancel_removes_only_the_matching_order() {
    let mut w = Warehouse::new();
    for id in 1..=3 {
        w.place_order(order(id, "x", 1)).unwrap();
    }
    assert_eq!(w.cancel(2), Some(order(2, "x", 1)));
    assert_eq!(queued_ids(&w), vec![1, 3], "remaining orders keep FIFO order");
    assert_eq!(w.cancel(99), None);
}

#[test]
fn part3_receive_saturates_instead_of_overflowing() {
    let mut w = Warehouse::new();
    w.receive("x", u32::MAX);
    w.receive("x", 5);
    assert_eq!(w.stock("x"), u32::MAX);
    assert_eq!(w.stock("unknown"), 0);
}

#[test]
fn part3_replace_backorders_returns_previous_queue() {
    let mut w = Warehouse::new();
    w.place_order(order(1, "x", 1)).unwrap();
    let old = w.replace_backorders(vec![order(9, "y", 1), order(10, "y", 1)]);
    assert_eq!(old.iter().map(|o| o.id).collect::<Vec<_>>(), vec![1]);
    assert_eq!(queued_ids(&w), vec![9, 10]);
}

#[test]
fn part3_split_pair_mut_gives_two_disjoint_elements() {
    let mut v = vec![1, 2, 3];
    if let Some((a, b)) = split_pair_mut(&mut v, 2, 0) {
        *a += 10;
        *b += 20;
    } else {
        panic!("indices 2 and 0 are valid and distinct");
    }
    assert_eq!(v, vec![21, 2, 13]);
}

#[test]
fn part3_split_pair_mut_rejects_equal_or_out_of_range() {
    let mut v = vec![1, 2, 3];
    assert!(split_pair_mut(&mut v, 1, 1).is_none(), "same index twice aliases");
    assert!(split_pair_mut(&mut v, 0, 3).is_none(), "out of bounds");
    let mut empty: Vec<i32> = Vec::new();
    assert!(split_pair_mut(&mut empty, 0, 1).is_none());
}

#[test]
fn part3_staged_publish_swaps_buffers() {
    let mut s = Staged::new(vec![1]);
    *s.staged_mut() = vec![2];
    assert_eq!(s.live(), &vec![1], "staged writes are invisible before publish");
    s.publish();
    assert_eq!(s.live(), &vec![2]);
    assert_eq!(s.staged_mut(), &vec![1], "staging buffer now holds the previous live value");
}

#[test]
fn part3_staged_starts_as_copy_of_initial_value() {
    let mut s = Staged::new(String::from("cfg"));
    assert_eq!(s.staged_mut().as_str(), "cfg");
    assert_eq!(s.live().as_str(), "cfg");
}
