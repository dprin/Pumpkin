use std::{cell::RefCell, fmt::Debug};

use itertools::Itertools;

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
        DomainEvents,
    },
    predicate,
    predicates::{Predicate, PropositionalConjunction},
    variables::IntegerVariable,
};

pub(crate) struct NotFirstNotLastPropagator<Var: IntegerVariable + Copy + Debug + 'static> {
    tasks: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + Debug + Copy + 'static> NotFirstNotLastPropagator<Var> {
    pub(crate) fn new(tasks: Vec<Task<Var>>) -> Self {
        Self { tasks }
    }

    fn not_last(&self, ctx: &RefCell<PropagationContextMut>) -> PropagationStatusCP {
        let mut context = ctx.borrow_mut();

        for i in &self.tasks {
            let nl_set: Vec<_> = self
                .tasks
                .clone()
                .into_iter()
                .filter(|j| {
                    j.get_lst(&context.assignments) < i.get_lct(&context.assignments)
                        && j.local_id != i.local_id
                })
                .collect();

            // dbg!(&nl_set
            //     .iter()
            //     .map(|x| x.get_est(&context.assignments))
            //     .collect::<Vec<_>>());

            if nl_set.is_empty() {
                continue;
            }

            let tree = Theta::new(nl_set.clone(), &context.assignments);

            if tree.get_ect() > i.get_lst(&context.assignments) {
                // println!(
                //     "tree: {:#?}\nwith lst {} and tree.ect {}",
                //     tree,
                //     t.get_lst(&context.assignments),
                //     tree.get_ect(),
                // );
                let biggest_task = nl_set
                    .into_iter()
                    .sorted_by(|a, b| {
                        let b = b.get_lst(&context.assignments);
                        let a = a.get_lst(&context.assignments);

                        b.cmp(&a)
                    })
                    .next()
                    .expect("Should not be empty because nl set was asserted not to be");

                let new_lst = biggest_task.get_lst(&context.assignments) - i.processing_time;

                let reason: PropositionalConjunction = self
                    .tasks
                    .iter()
                    .flat_map(|task| {
                        vec![
                            predicate![task.var >= task.var.lower_bound(&context.assignments)],
                            predicate![task.var <= task.var.upper_bound(&context.assignments)],
                        ]
                    })
                    .collect();
                // println!(
                //     "Setting {:?} from {:?} to {:?}",
                //     &t.local_id,
                //     &t.get_lst(&context.assignments),
                //     new_lst
                // );

                context.set_upper_bound(&i.var, new_lst, reason)?;
            }
        }

        Ok(())
    }
}

impl<Var: IntegerVariable + Copy + Debug + 'static> Propagator for NotFirstNotLastPropagator<Var> {
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
        init: &mut PropagatorInitialisationContext,
    ) -> Result<(), PropositionalConjunction> {
        for t in self.tasks.clone() {
            let _ = init.register(t.var, DomainEvents::BOUNDS, t.local_id);
        }

        let assignments = &init.assignments;

        self.tasks
            .sort_by(|a, b| a.get_est(assignments).cmp(&b.get_est(assignments)));

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

#[cfg(test)]
mod nl_tests {
    use crate::{
        constraints::Task,
        engine::{propagation::LocalId, test_solver::TestSolver},
        propagators::disjunctive::not_first_not_last::NotFirstNotLastPropagator,
    };

    #[test]
    fn check_propagate_large() {
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
            var: solver.new_variable(21, 30),
            processing_time: 5,
            local_id: LocalId::from(4),
        };
        let t5 = Task {
            var: solver.new_variable(24, 25),
            processing_time: 3,
            local_id: LocalId::from(5),
        };
        let t6 = Task {
            var: solver.new_variable(31, 40),
            processing_time: 20,
            local_id: LocalId::from(6),
        };

        let tasks = vec![t1, t2, t3, t4, t5, t6];

        let propagator = solver.new_propagator(NotFirstNotLastPropagator::new(tasks));
        // .expect("fail");

        // let result = solver.propagate(propagator);
        // println!("Propagation result: {:?}", result);
        println!(
            "t1 bounds: {} - {}",
            solver.lower_bound(t1.var),
            solver.upper_bound(t1.var)
        );
        println!(
            "t2 bounds: {} - {}",
            solver.lower_bound(t2.var),
            solver.upper_bound(t2.var)
        );
        println!(
            "t3 bounds: {} - {}",
            solver.lower_bound(t3.var),
            solver.upper_bound(t3.var)
        );
        println!(
            "t4 bounds: {} - {}",
            solver.lower_bound(t4.var),
            solver.upper_bound(t4.var)
        );
        println!(
            "t5 bounds: {} - {}",
            solver.lower_bound(t5.var),
            solver.upper_bound(t5.var)
        );
        println!(
            "t6 bounds: {} - {}",
            solver.lower_bound(t6.var),
            solver.upper_bound(t6.var)
        );
    }
}
