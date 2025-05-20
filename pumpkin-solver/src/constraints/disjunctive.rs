use super::Constraint;
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
    StartTimes::Item: IntegerVariable + Copy + 'static,
    StartTimes::IntoIter: ExactSizeIterator,
    Durations: IntoIterator<Item = i32>,
    Durations::IntoIter: ExactSizeIterator,
{
    let start_times = start_times.into_iter().collect::<Vec<_>>();
    let durations = durations.into_iter().collect::<Vec<_>>();

    pumpkin_assert_simple!(start_times.len() == durations.len());

    // Disjunctive::new(start_times, durations)
    let tasks: Vec<Task<StartTimes::Item>> = start_times
        .into_iter()
        .zip(durations.into_iter())
        .map(|(s, d)| Task {
            start_time: s,
            processing_time: d,
        })
        .collect();

    Disjunctive::<StartTimes::Item>::new(tasks)
}

/// Task variable which will store all tasks
pub(crate) struct Task<Var> {
    pub(crate) start_time: Var,
    pub(crate) processing_time: i32,
}

struct Disjunctive<Var: IntegerVariable + 'static> {
    tasks: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + 'static> Disjunctive<Var> {
    fn new(tasks: Vec<Task<Var>>) -> Self {
        Self { tasks }
    }
}

impl<Var: IntegerVariable + 'static> Constraint for Disjunctive<Var> {
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
