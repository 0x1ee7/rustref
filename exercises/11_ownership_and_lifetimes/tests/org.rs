use ex11_ownership_and_lifetimes::org::{Employee, OrgError};

/// ceo → cto → eng → dev, plus ceo → ops (salaries 300/200/100/50/50).
fn org() -> (Employee, Employee, Employee, Employee, Employee) {
    let ceo = Employee::new_root("ceo", 300);
    let cto = ceo.hire("cto", 200);
    let eng = cto.hire("eng", 100);
    let dev = eng.hire("dev", 50);
    let ops = ceo.hire("ops", 50);
    (ceo, cto, eng, dev, ops)
}

fn names(list: &[Employee]) -> Vec<&str> {
    list.iter().map(|e| e.name()).collect()
}

#[test]
fn part4_hire_links_manager_and_reports() {
    let root = Employee::new_root("ceo", 300);
    let cto = root.hire("cto", 200);
    assert!(cto.manager().unwrap().ptr_eq(&root));
    let reports = root.reports();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].ptr_eq(&cto));
}

#[test]
fn part4_chain_of_command_walks_to_root() {
    let (_ceo, _cto, _eng, dev, _ops) = org();
    assert_eq!(names(&dev.chain_of_command()), vec!["eng", "cto", "ceo"]);
}

#[test]
fn part4_total_cost_sums_subtree() {
    let (ceo, cto, _eng, _dev, _ops) = org();
    assert_eq!(cto.total_cost(), 200 + 100 + 50);
    assert_eq!(ceo.total_cost(), 300 + 200 + 100 + 50 + 50);
}

#[test]
fn part4_set_salary_is_visible_through_every_handle() {
    let (ceo, _cto, _eng, _dev, _ops) = org();
    let alias = ceo.clone();
    alias.set_salary(1);
    assert_eq!(ceo.salary(), 1);
    assert_eq!(ceo.reports()[0].manager().unwrap().salary(), 1);
}

#[test]
fn part4_find_is_depth_first_and_includes_self() {
    let (ceo, _cto, _eng, dev, _ops) = org();
    assert!(ceo.find("dev").unwrap().ptr_eq(&dev));
    assert!(ceo.find("ceo").unwrap().ptr_eq(&ceo));
    assert!(ceo.find("nobody").is_none());
}

#[test]
fn part4_weak_manager_link_does_not_keep_parent_alive() {
    let root = Employee::new_root("ceo", 1);
    let child = root.hire("cto", 1);
    drop(root);
    assert!(child.manager().is_none(), "the parent was freed; the child's link is weak");
    assert_eq!(child.strong_count(), 1, "only our handle remains");
}

#[test]
fn part4_strong_and_weak_counts_follow_links() {
    let root = Employee::new_root("ceo", 1);
    assert_eq!(root.strong_count(), 1);
    assert_eq!(root.weak_count(), 0);

    let child = root.hire("cto", 1);
    assert_eq!(root.strong_count(), 1, "child → parent is weak");
    assert_eq!(root.weak_count(), 1, "one weak link from the child");
    assert_eq!(child.strong_count(), 2, "our handle + root's reports");
}

#[test]
fn part4_dropping_the_tree_frees_every_node() {
    let root = Employee::new_root("ceo", 1);
    let cto = root.hire("cto", 1);
    let dev = cto.hire("dev", 1);
    let w_cto = cto.downgrade();
    let w_dev = dev.downgrade();
    drop(cto);
    drop(dev);
    drop(root);
    assert!(w_cto.upgrade().is_none(), "cto leaked: a cycle keeps it alive");
    assert!(w_dev.upgrade().is_none(), "dev leaked");
}

#[test]
fn part4_transfer_moves_subtree_and_costs() {
    let (ceo, cto, eng, _dev, ops) = org();
    eng.transfer_to(&ops).expect("eng moves under ops");
    assert_eq!(ops.total_cost(), 50 + 100 + 50, "ops + eng + dev");
    assert_eq!(cto.total_cost(), 200, "cto no longer has eng's subtree");
    assert!(cto.reports().is_empty());
    assert!(eng.manager().unwrap().ptr_eq(&ops));
    assert!(ceo.find("dev").is_some(), "dev moved with eng");
}

#[test]
fn part4_transfer_rejects_cycles_without_changing_the_tree() {
    let (ceo, cto, eng, _dev, _ops) = org();
    assert_eq!(cto.transfer_to(&eng), Err(OrgError::Cycle), "under own report");
    assert_eq!(cto.transfer_to(&cto), Err(OrgError::Cycle), "under itself");
    assert!(cto.manager().unwrap().ptr_eq(&ceo), "failed transfer must not move anything");
    assert_eq!(ceo.total_cost(), 300 + 200 + 100 + 50 + 50);
}

#[test]
fn part4_root_cannot_be_moved() {
    let (ceo, cto, _eng, _dev, _ops) = org();
    assert_eq!(ceo.transfer_to(&cto), Err(OrgError::RootCannotMove));
}

#[test]
fn part4_transfer_to_current_manager_is_a_noop() {
    let (ceo, cto, _eng, _dev, _ops) = org();
    assert_eq!(cto.transfer_to(&ceo), Ok(()));
    assert_eq!(ceo.reports().len(), 2, "no duplicate report entry");
}

#[test]
fn part4_transfer_keeps_a_single_owner() {
    let (_ceo, cto, eng, _dev, ops) = org();
    assert_eq!(eng.strong_count(), 2, "our handle + cto's reports");
    eng.transfer_to(&ops).unwrap();
    assert_eq!(eng.strong_count(), 2, "our handle + ops' reports, no leftover owner in cto");
    assert_eq!(names(&cto.reports()), Vec::<&str>::new());
}
