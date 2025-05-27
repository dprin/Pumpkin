use crate::engine::Assignments;
use crate::pumpkin_assert_simple;
use crate::variables::IntegerVariable;
use std::fmt::Debug;

use super::Task;

#[derive(Debug)]
pub(crate) struct Theta<Var>
where
    Var: IntegerVariable + Debug,
{
    /// tree structure
    nodes: Vec<ThetaNode<Var>>,
}

impl<Var> Theta<Var>
where
    Var: IntegerVariable + Debug + 'static,
{
    /// Creates a new Theta tree given amount of tasks.
    ///
    /// It uses the amount of tasks in order to prepare the vector.
    pub(crate) fn new(tasks: &mut Vec<Task<Var>>, assignments: &Assignments) -> Self {
        let len = tasks.len();
        pumpkin_assert_simple!(len != 0, "Size of theta tree can't be 0!");
        let amount = len.next_power_of_two();

        let mut nodes = vec![ThetaNode::default(); amount - 1];

        tasks.sort_by(|a, b| a.get_est(assignments).cmp(&b.get_est(assignments)));

        nodes.extend(tasks.iter().map(|x| {
            let var = x.var.clone();
            let p = x.processing_time;
            let est = var.lower_bound(assignments);
            let lct = var.upper_bound(assignments) + p;

            ThetaNode::Leaf {
                id: var,
                est,
                lct,
                ect: est + p,
                duration: p,
            }
        }));
        let mut dummy_nodes: Vec<ThetaNode<Var>> = Vec::with_capacity(amount - len);

        for _ in 0..(amount - len) {
            let id = tasks[0].var.clone();

            dummy_nodes.push(ThetaNode::Leaf {
                id,
                est: i32::MIN,
                ect: i32::MIN,
                lct: i32::MIN,
                duration: 0,
            });
        }

        let mut tree = Self { nodes };

        for i in (amount - 1)..(2 * amount - 1) {
            if i == 0 {
                continue;
            }
            tree.update(Self::parent(i));
        }

        tree
    }

    #[inline]
    fn parent(index: usize) -> usize {
        (index - 1) / 2
    }

    /// update the tree based on the index, and then update parent.
    fn update(&mut self, index: usize) {
        let left = self.nodes.get(index * 2 + 1).unwrap_or_default();
        let right = self.nodes.get(index * 2 + 2).unwrap_or_default();

        self.nodes[index] = ThetaNode::combine(&left, &right);

        if index != 0 {
            self.update((index - 1) / 2);
        }
    }

    pub(crate) fn get_ect(&self) -> i32 {
        if self.nodes.is_empty() {
            i32::MIN
        } else {
            self.nodes[0].get_ect()
        }
    }

    pub(crate) fn get_lct(&self) -> i32 {
        if self.nodes.is_empty() {
            i32::MIN
        } else {
            self.nodes[0].get_lct()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_duration(&self) -> i32 {
        if self.nodes.is_empty() {
            0
        } else {
            self.nodes[0].get_duration()
        }
    }
}

#[cfg(test)]
mod theta_tests {
    use crate::{
        engine::{propagation::LocalId, test_solver::TestSolver},
        variables::DomainId,
    };

    use super::*;

    #[test]
    fn one_task() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            var: solver.new_variable(0, 15),
            processing_time: 5,
            local_id: LocalId::from(1),
        };

        let t: Theta<DomainId> = Theta::new(&mut vec![t1], &solver.assignments);

        assert_eq!(t.get_ect(), 5);
        assert_eq!(t.get_duration(), 5);
    }

    #[test]
    fn example() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            var: solver.new_variable(0, 15),
            local_id: LocalId::from(1),
            processing_time: 5,
        };
        let t2 = Task {
            var: solver.new_variable(25, 31),
            local_id: LocalId::from(1),
            processing_time: 6,
        };
        let t3 = Task {
            local_id: LocalId::from(1),
            var: solver.new_variable(30, 34),
            processing_time: 4,
        };
        let t4 = Task {
            local_id: LocalId::from(1),
            var: solver.new_variable(32, 42),
            processing_time: 10,
        };

        let t: Theta<DomainId> = Theta::new(&mut vec![t1, t2, t3, t4], &solver.assignments);

        assert_eq!(t.get_ect(), 45);
        assert_eq!(t.get_duration(), 25);
    }

    #[test]
    fn three_nodes() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            var: solver.new_variable(0, 10),
            processing_time: 5,
            local_id: LocalId::from(1),
        };
        let t2 = Task {
            var: solver.new_variable(2, 4),
            processing_time: 3,
            local_id: LocalId::from(2),
        };
        let t3 = Task {
            var: solver.new_variable(11, 15),
            processing_time: 10,
            local_id: LocalId::from(3),
        };

        let t: Theta<DomainId> = Theta::new(&mut vec![t1, t2, t3], &solver.assignments);
        assert_eq!(t.get_ect(), 21);
    }

    #[test]
    fn two_nodes() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            var: solver.new_variable(0, 10),
            processing_time: 5,
            local_id: LocalId::from(1),
        };
        let t2 = Task {
            var: solver.new_variable(11, 15),
            processing_time: 10,
            local_id: LocalId::from(3),
        };
        let t: Theta<DomainId> = Theta::new(&mut vec![t1, t2], &solver.assignments);
        assert_eq!(t.get_ect(), 21);
    }

    #[test]
    fn five_nodes() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            var: solver.new_variable(0, 10),
            processing_time: 5,
            local_id: LocalId::from(1),
        };
        let t2 = Task {
            var: solver.new_variable(2, 4),
            processing_time: 3,
            local_id: LocalId::from(2),
        };
        let t3 = Task {
            var: solver.new_variable(11, 15),
            processing_time: 10,
            local_id: LocalId::from(3),
        };
        let t4 = Task {
            var: solver.new_variable(13, 24),
            processing_time: 2,
            local_id: LocalId::from(4),
        };
        let t5 = Task {
            var: solver.new_variable(14, 26),
            processing_time: 1,
            local_id: LocalId::from(5),
        };

        let t: Theta<DomainId> = Theta::new(&mut vec![t1, t2, t3, t4, t5], &solver.assignments);
        assert_eq!(t.get_ect(), 24);
    }
}

impl<Time> ThetaNode<Time>
where
    Time: IntegerVariable,
{
    fn get_duration(&self) -> i32 {
        match self {
            ThetaNode::Leaf { duration, .. } => duration.clone(),
            ThetaNode::Node { duration, .. } => duration.clone(),
        }
    }

    fn get_ect(&self) -> i32 {
        match self {
            ThetaNode::Leaf { ect, .. } => ect.clone(),
            ThetaNode::Node { ect, .. } => ect.clone(),
        }
    }

    fn get_lct(&self) -> i32 {
        match self {
            ThetaNode::Leaf { lct, .. } => lct.clone(),
            ThetaNode::Node { lct, .. } => lct.clone(),
        }
    }

    fn combine(left: &Self, right: &Self) -> Self {
        let duration = left.get_duration() + right.get_duration();
        let ect = i32::max(right.get_ect(), left.get_ect() + right.get_duration());
        let lct = i32::max(left.get_lct(), right.get_lct());
        ThetaNode::Node { duration, ect, lct }
    }
}

impl<Time> Default for ThetaNode<Time>
where
    Time: IntegerVariable,
{
    fn default() -> Self {
        Self::Node {
            duration: 0,
            lct: i32::MIN,
            ect: i32::MIN,
        }
    }
}

impl<Time> Default for &ThetaNode<Time>
where
    Time: IntegerVariable,
{
    fn default() -> Self {
        &ThetaNode::Node {
            duration: 0,
            lct: i32::MIN,
            ect: i32::MIN,
        }
    }
}

impl<Time> Clone for ThetaNode<Time>
where
    Time: IntegerVariable,
{
    fn clone(&self) -> Self {
        ThetaNode::default()
    }
}

#[derive(Debug)]
enum ThetaNode<Time>
where
    Time: IntegerVariable,
{
    Leaf {
        id: Time,
        est: i32,
        lct: i32,
        ect: i32,
        duration: i32,
    },
    Node {
        duration: i32,
        ect: i32,
        lct: i32,
    },
}
