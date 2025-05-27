use std::{cell::RefCell, fmt::Debug, fs::OpenOptions, io::Write};

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
    tasks_lst: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + Debug + Copy + 'static> NotFirstNotLastPropagator<Var> {
    pub(crate) fn new(tasks: Vec<Task<Var>>) -> Self {
        Self {
            tasks: tasks.clone(),
            tasks_lst: tasks,
        }
    }

    fn not_first(
        tasks: &Vec<Task<Var>>,
        ctx: &RefCell<PropagationContextMut>,
    ) -> PropagationStatusCP {
        let mut context = ctx.borrow_mut();
        let mut tasks = tasks.clone();
        tasks.sort_by(|a, b| {
            a.get_est(&context.assignments)
                .cmp(&b.get_est(&context.assignments))
        });

        let len = tasks.len();

        let reason: PropositionalConjunction = tasks
            .iter()
            .flat_map(|task| {
                vec![
                    predicate![task.var >= task.var.lower_bound(&context.assignments)],
                    predicate![task.var <= task.var.upper_bound(&context.assignments)],
                ]
            })
            .collect();

        for i in tasks.iter() {
            let est_i = i.get_est(&context.assignments);
            let mut max_lct = i32::MIN;
            let mut p = 0;
            let mut set: Vec<&Task<Var>> = Vec::with_capacity(len);

            for j in tasks.iter() {
                if est_i > j.get_est(&context.assignments) {
                    break;
                }

                if i.local_id == j.local_id {
                    continue;
                }

                max_lct = i32::max(max_lct, j.get_lct(&context.assignments));
                p += j.processing_time;

                set.push(j);
            }

            if set.is_empty() {
                continue;
            }

            if max_lct - est_i < p + i.processing_time {
                let mut min_ect = i32::MAX;

                for j in set {
                    min_ect = i32::min(min_ect, j.get_ect(&context.assignments));
                }

                if est_i > min_ect {
                    continue;
                }

                context.set_upper_bound(&i.var, min_ect, reason.clone())?;
            }
        }

        Ok(())
    }

    fn not_last(
        tasks: &Vec<Task<Var>>,
        tasks_lst: &Vec<Task<Var>>,
        ctx: &RefCell<PropagationContextMut>,
    ) -> PropagationStatusCP {
        // Create vec of old lsts that will be used to update the list
        let mut context = ctx.borrow_mut();
        let mut new_lsts: Vec<_> = tasks
            .iter()
            .map(|x| x.get_lst(&context.assignments))
            .collect();

        let reason: PropositionalConjunction = tasks
            .iter()
            .flat_map(|task| {
                vec![
                    predicate![task.var >= task.var.lower_bound(&context.assignments)],
                    predicate![task.var <= task.var.upper_bound(&context.assignments)],
                ]
            })
            .collect();

        for i in 0..tasks.len() {
            let i_task = tasks[i];
            let lct_i = i_task.get_lct(&context.assignments);

            // take all tasks that could be used
            let mut set: Vec<Task<Var>> = Vec::with_capacity(tasks.len());

            for j in tasks_lst {
                if lct_i <= j.get_lst(&context.assignments) {
                    break;
                }

                if i_task.local_id == j.local_id {
                    continue;
                }

                set.push(j.clone());
            }

            if set.is_empty() {
                continue;
            }

            let theta = Theta::new(&mut (set.clone()), &context.assignments);

            // if ECT_theta > LST_i
            if theta.get_ect() > i_task.get_lst(&context.assignments) {
                new_lsts[i] = i32::min(
                    set[0].get_lst(&context.assignments) - i_task.processing_time,
                    new_lsts[i],
                );
            }
        }

        // update all lsts
        for i in 0..new_lsts.len() {
            if tasks[i].get_lst(&context.assignments) == new_lsts[i] {
                continue;
            }

            assert!(
                tasks[i].get_lst(&context.assignments) > new_lsts[i],
                "Attempted to put a higher LST for a task"
            );

            context.set_upper_bound(&tasks[i].var, new_lsts[i], reason.clone())?;
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
        // let mut f = OpenOptions::new()
        //     .append(true)
        //     .create(true)
        //     .open("hahaha")
        //     .expect("could not open file");

        // let _ = f
        //     .write_all("before: ".as_bytes())
        //     .expect("could not newline");
        // f.flush();

        // for t in &self.tasks {
        //     let context = ctx.borrow();
        //     let _ = f
        //         .write_all(format!("{} ", t.get_lct(&context.assignments)).as_bytes())
        //         .expect("could not write_all");
        // }
        // let _ = f
        //     .write_all("\nafter: ".as_bytes())
        //     .expect("could not newline");
        // f.flush();

        Self::not_first(&self.tasks, &ctx)?;
        Self::not_last(&self.tasks, &self.tasks_lst, &ctx)?;

        // for t in &self.tasks {
        //     let context = ctx.borrow();
        //     let _ = f
        //         .write_all(format!("{} ", t.get_lct(&context.assignments)).as_bytes())
        //         .expect("could not write");
        // }

        // let _ = f.write_all("\n".as_bytes()).expect("could not newline");
        // f.flush();

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
            .sort_by(|a, b| a.get_lct(assignments).cmp(&b.get_lct(assignments)));
        self.tasks_lst
            .sort_by(|a, b| a.get_lst(assignments).cmp(&b.get_lst(assignments)));

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

        let _propagator = solver.new_propagator(NotFirstNotLastPropagator::new(tasks));
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
