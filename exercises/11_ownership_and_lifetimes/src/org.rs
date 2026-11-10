//! Part 4 — an org chart with `Rc<Node>`, `RefCell` children and `Weak` parent links.

/// Strong handle to an employee. Cloning is a reference-count bump.
#[derive(Clone)]
pub struct Employee {
    // engineer: Rc<Node>
}

/// Non-owning handle; does not keep the employee alive.
#[derive(Clone)]
pub struct WeakEmployee {
    // engineer: Weak<Node>
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
    /// A top-level employee with no manager.
    pub fn new_root(_name: impl Into<String>, _salary: u32) -> Employee {
        todo!()
    }

    /// Creates a new report of `self`.
    pub fn hire(&self, _name: impl Into<String>, _salary: u32) -> Employee {
        todo!()
    }

    /// The name, borrowed from `self`.
    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn salary(&self) -> u32 {
        todo!()
    }

    /// Visible through every handle to this employee.
    pub fn set_salary(&self, _salary: u32) {
        todo!()
    }

    /// Direct manager, if still alive.
    pub fn manager(&self) -> Option<Employee> {
        todo!()
    }

    /// Direct reports, as new handles, in hiring order.
    pub fn reports(&self) -> Vec<Employee> {
        todo!()
    }

    /// Managers from the direct manager up to the root (excludes `self`).
    pub fn chain_of_command(&self) -> Vec<Employee> {
        todo!()
    }

    /// Sum of salaries of `self` and every descendant.
    pub fn total_cost(&self) -> u64 {
        todo!()
    }

    /// Depth-first search of `self` and its descendants by exact name.
    pub fn find(&self, _name: &str) -> Option<Employee> {
        todo!()
    }

    /// Moves `self` (with its whole subtree) under `new_manager`.
    ///
    /// Moving to the current manager is `Ok` and changes nothing.
    pub fn transfer_to(&self, _new_manager: &Employee) -> Result<(), OrgError> {
        todo!()
    }

    pub fn downgrade(&self) -> WeakEmployee {
        todo!()
    }

    /// Same allocation?
    pub fn ptr_eq(&self, _other: &Employee) -> bool {
        todo!()
    }

    /// Number of strong handles to this employee's node.
    pub fn strong_count(&self) -> usize {
        todo!()
    }

    /// Number of weak handles to this employee's node.
    pub fn weak_count(&self) -> usize {
        todo!()
    }
}

impl WeakEmployee {
    /// A strong handle, if the employee is still alive.
    pub fn upgrade(&self) -> Option<Employee> {
        todo!()
    }
}
