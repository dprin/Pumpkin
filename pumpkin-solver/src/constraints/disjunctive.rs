use std::fmt::Debug;
use std::hash::Hash;

use super::Constraint;
use crate::engine::propagation::LocalId;
use crate::engine::Assignments;
use crate::propagators::disjunctive::not_first_not_last::NotFirstNotLastPropagator;
use crate::pumpkin_assert_simple;
use crate::variables::IntegerVariable;

/// Creates the [Disjunctive](https://sofdem.github.io/gccat/gccat/Cdisjunctive.html) [`Constraint`].
///
/// This constraint ensures that at no point in time the provided task can overlap. This can be
/// seen as a special case of the `cumulative` constraint with capacity 1.
///
/// The length of `start_times` and `durations` should be the same; if
/// this is not the case then this method will panic.
pub fn disjunctive<StartTimes, Durations>(
    start_times: StartTimes,
    durations: Durations,
) -> impl Constraint
where
    StartTimes: IntoIterator,
    StartTimes::Item: IntegerVariable + Eq + Hash + Copy + Debug + 'static,
    StartTimes::IntoIter: ExactSizeIterator,
    Durations: IntoIterator<Item = i32>,
    Durations::IntoIter: ExactSizeIterator,
{
    let start_times = start_times.into_iter().collect::<Vec<_>>();
    let durations = durations.into_iter().collect::<Vec<_>>();

    pumpkin_assert_simple!(start_times.len() == durations.len());

    // Disjunctive::new(start_times, durations)

    let pre_tasks: Vec<(StartTimes::Item, i32)> =
        start_times.into_iter().zip(durations.into_iter()).collect();
    let mut tasks: Vec<Task<StartTimes::Item>> = Vec::new();

    for i in 0..pre_tasks.len() {
        tasks.push(Task {
            var: pre_tasks[i].0,
            processing_time: pre_tasks[i].1,
            local_id: LocalId::from(i as u32),
        });
    }

    Disjunctive::<StartTimes::Item>::new(tasks)
}

/// Task variable which will store all tasks
#[derive(PartialEq, Debug, Eq, Clone, Copy, Hash)]
pub(crate) struct Task<Var: IntegerVariable + Debug> {
    pub(crate) var: Var,
    pub(crate) processing_time: i32,
    pub(crate) local_id: LocalId,
}

impl<Var: IntegerVariable + Debug> Task<Var> {
    pub(crate) fn get_est(&self, assignments: &Assignments) -> i32 {
        self.var.lower_bound(assignments)
    }

    pub(crate) fn get_lct(&self, assignments: &Assignments) -> i32 {
        self.var.upper_bound(assignments) + self.processing_time
    }

    pub(crate) fn get_lst(&self, assignments: &Assignments) -> i32 {
        self.var.upper_bound(assignments)
    }

    pub(crate) fn to_string(&self, assignments: &Assignments) -> String {
        format!(
            "Task {:?}: {} - {}",
            self.var,
            self.get_est(assignments),
            self.get_lst(assignments)
        )
    }
}

struct Disjunctive<Var: IntegerVariable + Eq + Hash + Copy + 'static> {
    tasks: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + Copy + Eq + Hash + 'static> Disjunctive<Var> {
    fn new(tasks: Vec<Task<Var>>) -> Self {
        Self { tasks }
    }
}

impl<Var: IntegerVariable + Copy + Eq + Hash + Debug + 'static> Constraint for Disjunctive<Var> {
    fn post(
        self,
        solver: &mut crate::Solver,
        tag: Option<std::num::NonZero<u32>>,
    ) -> Result<(), crate::ConstraintOperationError> {
        NotFirstNotLastPropagator::new(self.tasks).post(solver, tag)
    }

    fn implied_by(
        self,
        solver: &mut crate::Solver,
        reification_literal: crate::variables::Literal,
        tag: Option<std::num::NonZero<u32>>,
    ) -> Result<(), crate::ConstraintOperationError> {
        NotFirstNotLastPropagator::new(self.tasks).implied_by(solver, reification_literal, tag)
    }
}
