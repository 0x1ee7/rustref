//! Part 4 — an org chart with `Rc`, `RefCell`, `Cell` and `Weak` (`src/org.rs`).
//!
//! Reports own their manager's children strongly; the manager link is `Weak`,
//! so the tree has no reference cycle and dropping the root frees everyone.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct Node {
    name: String,
    salary: Cell<u32>,
    manager: RefCell<Weak<Node>>,
    reports: RefCell<Vec<Employee>>,
}

/// Strong handle to an employee. Cloning is a reference-count bump.
#[derive(Clone)]
pub struct Employee {
    node: Rc<Node>,
}

/// Non-owning handle; does not keep the employee alive.
#[derive(Clone)]
pub struct WeakEmployee {
    node: Weak<Node>,
}

/// Errors from [`Employee::transfer_to`].
#[derive(Debug, PartialEq, Eq)]
pub enum OrgError {
    /// The root of the org chart has no manager to move away from.
    RootCannotMove,
    /// The new manager is the employee itself or one of their reports.
    Cycle,
}

impl Employee {
    fn from_node(node: Node) -> Employee {
        Employee { node: Rc::new(node) }
    }

    /// A top-level employee with no manager.
    pub fn new_root(name: impl Into<String>, salary: u32) -> Employee {
        Employee::from_node(Node {
            name: name.into(),
            salary: Cell::new(salary),
            manager: RefCell::new(Weak::new()),
            reports: RefCell::new(Vec::new()),
        })
    }

    /// Creates a new report of `self`.
    pub fn hire(&self, name: impl Into<String>, salary: u32) -> Employee {
        let child = Employee::from_node(Node {
            name: name.into(),
            salary: Cell::new(salary),
            manager: RefCell::new(Rc::downgrade(&self.node)),
            reports: RefCell::new(Vec::new()),
        });
        self.node.reports.borrow_mut().push(child.clone());
        child
    }

    /// The name, borrowed from `self`.
    pub fn name(&self) -> &str {
        &self.node.name
    }

    pub fn salary(&self) -> u32 {
        self.node.salary.get()
    }

    /// Visible through every handle to this employee.
    pub fn set_salary(&self, salary: u32) {
        self.node.salary.set(salary);
    }

    /// Direct manager, if still alive.
    pub fn manager(&self) -> Option<Employee> {
        let node = self.node.manager.borrow().upgrade()?;
        Some(Employee { node })
    }

    /// Direct reports, as new handles, in hiring order.
    pub fn reports(&self) -> Vec<Employee> {
        self.node.reports.borrow().clone()
    }

    /// Managers from the direct manager up to the root (excludes `self`).
    pub fn chain_of_command(&self) -> Vec<Employee> {
        let mut chain = Vec::new();
        let mut current = self.manager();
        while let Some(manager) = current {
            current = manager.manager();
            chain.push(manager);
        }
        chain
    }

    /// Sum of salaries of `self` and every descendant.
    pub fn total_cost(&self) -> u64 {
        let reports = self.node.reports.borrow();
        u64::from(self.salary()) + reports.iter().map(Employee::total_cost).sum::<u64>()
    }

    /// Depth-first search of `self` and its descendants by exact name.
    pub fn find(&self, name: &str) -> Option<Employee> {
        if self.name() == name {
            return Some(self.clone());
        }
        let reports = self.node.reports.borrow();
        reports.iter().find_map(|r| r.find(name))
    }

    /// Moves `self` (with its whole subtree) under `new_manager`.
    ///
    /// Moving to the current manager is `Ok` and changes nothing.
    pub fn transfer_to(&self, new_manager: &Employee) -> Result<(), OrgError> {
        if self.manager().is_none() {
            return Err(OrgError::RootCannotMove);
        }
        // `new_manager` is inside our subtree iff walking up from it reaches us.
        let mut ancestor = Some(new_manager.clone());
        while let Some(a) = ancestor {
            if a.ptr_eq(self) {
                return Err(OrgError::Cycle);
            }
            ancestor = a.manager();
        }
        if self.manager().is_some_and(|m| m.ptr_eq(new_manager)) {
            return Ok(());
        }

        if let Some(old) = self.manager() {
            old.node.reports.borrow_mut().retain(|r| !r.ptr_eq(self));
        }
        *self.node.manager.borrow_mut() = Rc::downgrade(&new_manager.node);
        new_manager.node.reports.borrow_mut().push(self.clone());
        Ok(())
    }

    pub fn downgrade(&self) -> WeakEmployee {
        WeakEmployee {
            node: Rc::downgrade(&self.node),
        }
    }

    /// Same allocation?
    pub fn ptr_eq(&self, other: &Employee) -> bool {
        Rc::ptr_eq(&self.node, &other.node)
    }

    /// Number of strong handles to this employee's node.
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.node)
    }

    /// Number of weak handles to this employee's node.
    pub fn weak_count(&self) -> usize {
        Rc::weak_count(&self.node)
    }
}

impl WeakEmployee {
    /// A strong handle, if the employee is still alive.
    pub fn upgrade(&self) -> Option<Employee> {
        self.node.upgrade().map(|node| Employee { node })
    }
}
