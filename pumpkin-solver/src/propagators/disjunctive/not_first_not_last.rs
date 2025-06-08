use std::{cell::RefCell, collections::VecDeque, fmt::Debug, hash::Hash};

use crate::{
    basic_types::{Inconsistency, PropagationStatusCP},
    constraints::{theta::Theta, Task},
    engine::{
        opaque_domain_event::OpaqueDomainEvent,
        propagation::{
            contexts::PropagationContextWithTrailedValues, EnqueueDecision, ExplanationContext,
            LocalId, PropagationContext, PropagationContextMut, Propagator,
            PropagatorInitialisationContext,
        },
        Assignments, DomainEvents,
    },
    predicate,
    predicates::{Predicate, PropositionalConjunction},
    variables::{IntegerVariable, TransformableVariable},
};

#[allow(dead_code)]
enum ExplanationStrategy {
    Naive,
    Normal,
    Advanced,
}

const STRATEGY: ExplanationStrategy = ExplanationStrategy::Normal;
const MAX_SIZE_TO_SUBSET: usize = 10;

// copied from https://stackoverflow.com/questions/40718975/how-to-get-every-subset-of-a-vector-in-rust
fn powerset<T: Copy>(s: &Vec<T>) -> Vec<Vec<T>> {
    (0..2usize.pow(s.len() as u32))
        .map(|i| {
            s.iter()
                .enumerate()
                .filter(|&(t, _)| (i >> t) % 2 == 1)
                .map(|(_, element)| element)
                .copied()
                .collect()
        })
        .collect()
}

fn overload_check<Var: IntegerVariable + Copy + Debug + Eq + 'static>(
    tasks: Vec<Task<Var>>,
    assignments: &Assignments,
) -> Result<(), Inconsistency> {
    if tasks.is_empty() {
        return Ok(());
    }

    let mut est = i32::MAX;
    let mut lct = i32::MIN;
    let mut p = 0;

    for task in &tasks {
        est = i32::min(est, task.get_est(assignments));
        lct = i32::max(lct, task.get_lct(assignments));

        p += task.processing_time;
    }

    if i32::abs(est - lct) < p {
        // eprintln!("caught an overload!");
        return Err(Inconsistency::Conflict(naive_reason(assignments, &tasks)));
    }

    let next: Vec<_> = tasks.iter().cloned().skip(1).collect();

    overload_check(next, assignments)
}

fn handle_set<'a, Var: IntegerVariable + Copy + Debug + 'static>(
    tasks: Vec<Task<Var>>,
    i: &'a Task<Var>,
    assignments: &Assignments,
) -> Vec<Task<Var>> {
    match STRATEGY {
        ExplanationStrategy::Advanced => {
            if tasks.len() > MAX_SIZE_TO_SUBSET {
                return tasks;
            }

            let mut current_best = (tasks.clone(), i32::MAX);

            for subset in powerset(&tasks).iter().cloned() {
                if subset.is_empty() {
                    continue;
                }

                let max_lst = subset
                    .iter()
                    .max_by_key(|x| x.get_lst(assignments))
                    .unwrap()
                    .get_lst(assignments);

                let theta = Theta::new(&mut subset.clone(), assignments);

                if theta.get_ect() > i.get_lst(assignments) && max_lst < current_best.1 {
                    current_best = (subset, max_lst);
                }
            }

            current_best.0
        }
        _ => tasks,
    }
}

#[inline]
fn naive_reason<'a, Var: IntegerVariable + Eq + Copy + Debug + 'static>(
    assignments: &'a Assignments,
    tasks: &'a Vec<Task<Var>>,
) -> PropositionalConjunction {
    tasks
        .iter()
        .flat_map(|task| {
            vec![
                predicate![task.var >= task.var.lower_bound(assignments)],
                predicate![task.var <= task.var.upper_bound(assignments)],
            ]
        })
        .collect()
}

#[inline]
fn generate_reason<'a, Var: IntegerVariable + Eq + Copy + Debug + 'static>(
    all_tasks: &'a Vec<Task<Var>>,
    tasks: &'a Vec<Task<Var>>,
    i: &'a Task<Var>,
    assignments: &'a Assignments,
    new_value: i32,
) -> Result<PropositionalConjunction, Inconsistency> {
    match STRATEGY {
        ExplanationStrategy::Naive => Ok(naive_reason(assignments, all_tasks)),
        _ => {
            let mut tasks = tasks.clone();
            tasks.sort_by_key(|x| x.get_est(assignments));

            loop {
                let th = Theta::new(&mut (tasks.clone()), assignments);

                if th.get_ect() == th.get_est() + th.get_duration() {
                    break;
                }

                tasks = tasks[1..].to_vec();
            }

            let theta = Theta::new(&mut (tasks.clone()), assignments);

            let lower = i.get_lst(assignments) - theta.get_duration() + 1;
            let upper = new_value + i.processing_time;

            let mut reason: PropositionalConjunction = tasks
                .iter()
                .flat_map(|task| -> Result<Vec<Predicate>, Inconsistency> {
                    let mut ret: Vec<Predicate> = Vec::with_capacity(2);

                    ret.push(predicate![task.var >= lower]);
                    ret.push(predicate![task.var <= upper]);

                    Ok(ret)
                })
                .flatten()
                .collect();

            reason.add(predicate![i.var <= i.get_lst(assignments)]);

            Ok(reason)
        }
    }
}

pub(crate) struct NotFirstNotLastPropagator<
    Var: IntegerVariable + Hash + Eq + Copy + Debug + 'static,
> {
    tasks: Vec<Task<Var>>,
    tasks_lst: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + Debug + Eq + Hash + Copy + 'static> NotFirstNotLastPropagator<Var> {
    pub(crate) fn new(tasks: Vec<Task<Var>>) -> Self {
        Self {
            tasks: tasks.clone(),
            tasks_lst: tasks,
        }
    }
}

fn not_last<Var: IntegerVariable + Debug + Eq + Hash + Copy + Debug + 'static>(
    tasks: &mut Vec<Task<Var>>,
    tasks_lst: &mut Vec<Task<Var>>,
    ctx: &RefCell<PropagationContextMut>,
) -> PropagationStatusCP {
    let mut context = ctx.borrow_mut();

    tasks.sort_by_key(|x| x.get_lct(&context.assignments));
    tasks_lst.sort_by_key(|x| x.get_lst(&context.assignments));

    // Create vec of old lsts that will be used to update the list
    let mut new_lsts: Vec<(i32, PropositionalConjunction)> = tasks
        .iter()
        .map(|x| (x.get_lst(&context.assignments), vec![].into()))
        .collect();

    for i_ind in 0..tasks.len() {
        let i = tasks[i_ind];
        let lct_i = i.get_lct(&context.assignments);

        // take all tasks that could be used
        let mut set: Vec<Task<Var>> = Vec::with_capacity(tasks.len());
        let mut queue: VecDeque<_> = tasks_lst.into_iter().collect();

        let mut j: Option<&Task<Var>> = None;

        // while lct_i > lst of q_first
        while !queue.is_empty() && lct_i > queue[0].get_lst(&context.assignments) {
            if queue[0].local_id == i.local_id {
                let _ = queue.pop_front().unwrap();
                continue;
            }
            j = Some(queue.pop_front().unwrap());

            set.push(*j.unwrap());
        }

        if set.is_empty() {
            continue;
        }

        let mut set = handle_set(set, &i, &context.assignments);
        let theta = Theta::new(&mut set, &context.assignments);

        // if ECT_theta > LST_i
        // if Theta::(set, &context.assignments) > i_task.get_lst(&context.assignments) {
        if theta.get_ect() > i.get_lst(&context.assignments) {
            overload_check(set.clone(), &context.assignments)?;

            let new_lst = i32::min(
                new_lsts[i_ind].0,
                j.unwrap().get_lst(&context.assignments) - i.processing_time,
            );

            let reason = generate_reason(&tasks, &set, &i, &context.assignments, new_lst)?;
            new_lsts[i_ind] = (new_lst, reason);
        }
    }

    // update all lsts
    for i in 0..new_lsts.len() {
        let task = tasks[i];

        assert!(
            task.get_lst(&context.assignments) >= new_lsts[i].0,
            "Attempted to put a higher LST for a task"
        );

        context.set_upper_bound(&task.var, new_lsts[i].0, new_lsts[i].1.clone())?;
    }

    Ok(())
}

impl<Var: IntegerVariable + Copy + Hash + Eq + Debug + 'static> Propagator
    for NotFirstNotLastPropagator<Var>
{
    fn name(&self) -> &str {
        "Not-First/Not-Last"
    }

    fn debug_propagate_from_scratch(&self, context: PropagationContextMut) -> PropagationStatusCP {
        let ctx = RefCell::new(context);

        let context = ctx.borrow();
        let mut tasks = self.tasks.clone();
        let mut tasks_lst = self.tasks_lst.clone();

        let mut rev_tasks: Vec<_> = self
            .tasks
            .iter()
            .cloned()
            .map(
                |Task {
                     var,
                     processing_time,
                     local_id,
                 }| {
                    Task {
                        var: var.offset(processing_time).scaled(-1),
                        processing_time,
                        local_id,
                    }
                },
            )
            .collect();
        let mut rev_sorted: Vec<_> = self
            .tasks_lst
            .iter()
            .cloned()
            .map(
                |Task {
                     var,
                     processing_time,
                     local_id,
                 }| {
                    Task {
                        var: var.offset(processing_time).scaled(-1),
                        processing_time,
                        local_id,
                    }
                },
            )
            .collect();

        drop(context);

        not_last(&mut tasks, &mut tasks_lst, &ctx)?;
        not_last(&mut rev_tasks, &mut rev_sorted, &ctx)?;

        Ok(())
    }

    fn initialise_at_root(
        &mut self,
        init: &mut PropagatorInitialisationContext,
    ) -> Result<(), PropositionalConjunction> {
        for t in self.tasks.clone() {
            let _ = init.register(t.var, DomainEvents::BOUNDS, t.local_id);
        }

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
