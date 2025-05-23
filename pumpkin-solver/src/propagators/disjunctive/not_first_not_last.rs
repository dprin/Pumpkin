use std::cell::RefCell;

use crate::{
    basic_types::PropagationStatusCP,
    constraints::{theta::Theta, Task},
    engine::{
        opaque_domain_event::OpaqueDomainEvent,
        propagation::{
            contexts::PropagationContextWithTrailedValues, EnqueueDecision, ExplanationContext,
            LocalId, PropagationContext, PropagationContextMut, Propagator,
            PropagatorInitialisationContext,
        },
    },
    predicate,
    predicates::{Predicate, PropositionalConjunction},
    variables::IntegerVariable,
};

pub(crate) struct NotFirstNotLastPropagator<Var: IntegerVariable + Copy + 'static> {
    tasks: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + Copy + 'static> NotFirstNotLastPropagator<Var> {
    pub(crate) fn new(tasks: Vec<Task<Var>>) -> Self {
        Self { tasks }
    }

    fn not_last(&self, ctx: &RefCell<PropagationContextMut>) -> PropagationStatusCP {
        let mut context = ctx.borrow_mut();

        for t in &self.tasks {
            let nl_set: Vec<_> = self
                .tasks
                .clone()
                .into_iter()
                .filter(|x| {
                    x.get_lct(&context.assignments) - x.processing_time
                        < t.get_lct(&context.assignments)
                        && x.start_time.domain_id() != t.start_time.domain_id()
                })
                .collect();

            if nl_set.is_empty() {
                continue;
            }

            let tree = Theta::new(nl_set.clone(), &context.assignments);

            if tree.get_ect() > t.get_lct(&context.assignments) - t.processing_time {
                let new_lct = nl_set
                    .into_iter()
                    .map(|x| x.get_lct(&context.assignments) - x.processing_time)
                    .max()
                    .unwrap_or(t.get_lct(&context.assignments));

                let reason: PropositionalConjunction = self
                    .tasks
                    .iter()
                    .flat_map(|task| {
                        vec![
                            predicate![task.start_time >= task.get_est(&context.assignments)],
                            predicate![task.start_time <= task.get_lct(&context.assignments)],
                        ]
                    })
                    .collect();

                context.set_upper_bound(&t.start_time, new_lct, reason)?;
            }
        }

        Ok(())
    }

    /// Propagator for not first
    ///
    /// returns whether there was a change in bounds
    #[allow(dead_code)]
    fn not_first(&self, ctx: &RefCell<PropagationContextMut>) -> PropagationStatusCP {
        assert!(self
            .tasks
            .iter()
            .is_sorted_by(|a, b| a.processing_time >= b.processing_time));

        let mut context = ctx.borrow_mut();

        for i in 0..self.tasks.len() {
            let est_i = self.tasks[i].get_est(&context.assignments);
            let mut lst = i32::MAX;
            let mut eft = i32::MAX;

            for j in 0..self.tasks.len() {
                let est_j = self.tasks[j].get_est(&context.assignments);
                let lct_j = self.tasks[j].get_lct(&context.assignments);

                if est_j + self.tasks[j].processing_time <= est_i || i == j {
                    continue;
                }

                lst = i32::min(lct_j, lst) - self.tasks[j].processing_time;
                eft = i32::min(est_j + self.tasks[j].processing_time, eft);

                if lst >= est_i + self.tasks[i].processing_time {
                    continue;
                }

                let reason: PropositionalConjunction = self
                    .tasks
                    .iter()
                    .flat_map(|task| {
                        vec![
                            predicate![task.start_time >= task.get_est(&context.assignments)],
                            predicate![task.start_time <= task.get_lct(&context.assignments)],
                        ]
                    })
                    .collect();

                context.set_lower_bound(&self.tasks[i].start_time, eft, reason)?;

                break;
            }
        }
        Ok(())
    }
}

impl<Var: IntegerVariable + Copy + 'static> Propagator for NotFirstNotLastPropagator<Var> {
    fn name(&self) -> &str {
        "Not-First/Not-Last"
    }

    fn debug_propagate_from_scratch(&self, context: PropagationContextMut) -> PropagationStatusCP {
        let ctx = RefCell::new(context);
        self.not_last(&ctx)?;

        Ok(())
    }

    fn initialise_at_root(
        &mut self,
        _: &mut PropagatorInitialisationContext,
    ) -> Result<(), PropositionalConjunction> {
        // self.tasks
        //     .sort_by(|a, b| b.processing_time.cmp(&a.processing_time));

        Ok(())
    }

    fn propagate(&mut self, context: PropagationContextMut) -> PropagationStatusCP {
        self.debug_propagate_from_scratch(context)
    }

    fn notify(
        &mut self,
        _context: PropagationContextWithTrailedValues,
        _local_id: LocalId,
        _event: OpaqueDomainEvent,
    ) -> EnqueueDecision {
        EnqueueDecision::Enqueue
    }

    fn notify_backtrack(
        &mut self,
        _context: PropagationContext,
        _local_id: LocalId,
        _event: OpaqueDomainEvent,
    ) {
    }

    fn synchronise(&mut self, _context: PropagationContext) {}

    fn priority(&self) -> u32 {
        // setting an arbitrary priority by default
        3
    }

    fn detect_inconsistency(
        &self,
        _context: PropagationContextWithTrailedValues,
    ) -> Option<PropositionalConjunction> {
        None
    }

    fn lazy_explanation(&mut self, _code: u64, _context: ExplanationContext) -> &[Predicate] {
        std::panic!(
            "{}",
            format!(
                "Propagator {} does not support lazy explanations.",
                self.name()
            )
        );
    }

    fn log_statistics(&self, _statistic_logger: crate::statistics::StatisticLogger) {}
}
