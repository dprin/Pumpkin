use crate::engine::Assignments;
use crate::pumpkin_assert_simple;
use crate::variables::IntegerVariable;
use std::fmt::Debug;

use super::Task;

#[derive(Debug)]
pub(crate) struct Theta<Var>
where
    Var: IntegerVariable,
{
    /// tree structure
    nodes: Vec<ThetaNode<Var>>,
}

impl<Var> Theta<Var>
where
    Var: IntegerVariable + 'static,
{
    /// Creates a new Theta tree given amount of tasks.
    ///
    /// It uses the amount of tasks in order to prepare the vector.
    pub(crate) fn new(tasks: Vec<Task<Var>>, assignments: &Assignments) -> Self {
        let amount = tasks.len();
        pumpkin_assert_simple!(amount != 0, "Size of theta tree can't be 0!");

        let mut nodes = vec![ThetaNode::default(); amount - 1];

        nodes.extend(tasks.into_iter().map(|x| {
            let var = x.start_time.clone();
            let est = var.lower_bound(assignments);
            let p = x.processing_time;

            ThetaNode::Leaf {
                id: var,
                est,
                ect: est + p,
                duration: p,
            }
        }));

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

    // /// inserts a new node into the tree.
    // pub(crate) fn insert(&mut self, task: &Task<Var>, context: PropagationContext) {
    //     let est = context.lower_bound(&task.start_time);

    //     let insert = if self.get_ect() == i32::MIN {
    //         self.nodes.len()
    //     } else {
    //         // TODO: figure out how to do this in O(log n)

    //         // find start location
    //         // return that
    //         let mut i = self.leaves_loc;

    //         for ind in self.leaves_loc..self.nodes.len() {
    //             match self.nodes[ind] {
    //                 ThetaNode::Leaf { est: node_est, .. } => {
    //                     i = ind;
    //                     if est >= node_est {
    //                         break;
    //                     }
    //                 }
    //                 _ => panic!("Should not have a node"),
    //             }
    //         }

    //         i + 1
    //     };

    //     self.nodes.insert(
    //         insert,
    //         ThetaNode::Leaf {
    //             id: task.start_time,
    //             est,
    //             ect: est + task.processing_time,
    //             duration: task.processing_time,
    //         },
    //     );

    //     // update nodes up
    //     if insert != 0 {
    //         self.update(Self::parent(insert));
    //     }
    // }

    pub(crate) fn get_ect(&self) -> i32 {
        if self.nodes.is_empty() {
            i32::MIN
        } else {
            self.nodes[0].get_ect()
        }
    }

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
    use crate::{engine::test_solver::TestSolver, variables::DomainId};

    use super::*;

    #[test]
    fn one_task() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            start_time: solver.new_variable(0, 15),
            processing_time: 5,
        };

        let t: Theta<DomainId> = Theta::new(vec![t1], &solver.assignments);

        assert_eq!(t.get_ect(), 5);
        assert_eq!(t.get_duration(), 5);
    }

    #[test]
    fn example() {
        let mut solver = TestSolver::default();
        let t1 = Task {
            start_time: solver.new_variable(0, 15),
            processing_time: 5,
        };
        let t2 = Task {
            start_time: solver.new_variable(25, 31),
            processing_time: 6,
        };
        let t3 = Task {
            start_time: solver.new_variable(30, 34),
            processing_time: 4,
        };
        let t4 = Task {
            start_time: solver.new_variable(32, 42),
            processing_time: 10,
        };

        let t: Theta<DomainId> = Theta::new(vec![t1, t2, t3, t4], &solver.assignments);
        dbg!(&t);

        assert_eq!(t.get_ect(), 45);
        assert_eq!(t.get_duration(), 25);
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

    fn combine(left: &Self, right: &Self) -> Self {
        let duration = left.get_duration() + right.get_duration();
        let ect = i32::max(right.get_ect(), left.get_ect() + right.get_duration());

        ThetaNode::Node { duration, ect }
    }
}

impl<Time> Default for ThetaNode<Time>
where
    Time: IntegerVariable,
{
    fn default() -> Self {
        Self::Node {
            duration: 0,
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
        ect: i32,
        duration: i32,
    },
    Node {
        duration: i32,
        ect: i32,
    },
}
