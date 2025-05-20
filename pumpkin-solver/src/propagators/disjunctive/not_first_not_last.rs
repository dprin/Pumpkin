use crate::{
    basic_types::PropagationStatusCP,
    constraints::Task,
    engine::{
        opaque_domain_event::OpaqueDomainEvent,
        propagation::{
            contexts::PropagationContextWithTrailedValues, EnqueueDecision, ExplanationContext,
            LocalId, PropagationContext, PropagationContextMut, Propagator,
            PropagatorInitialisationContext,
        },
    },
    predicates::{Predicate, PropositionalConjunction},
    variables::IntegerVariable,
};

pub(crate) struct NotFirstNotLastPropagator<Var: IntegerVariable + 'static> {
    tasks: Vec<Task<Var>>,
}

impl<Var: IntegerVariable + 'static> NotFirstNotLastPropagator<Var> {
    pub(crate) fn new(tasks: Vec<Task<Var>>) -> Self {
        Self { tasks }
    }
}

impl<Var: IntegerVariable + 'static> Propagator for NotFirstNotLastPropagator<Var> {
    fn name(&self) -> &str {
        "Not-First/Not-Last"
    }

    fn debug_propagate_from_scratch(&self, context: PropagationContextMut) -> PropagationStatusCP {
        todo!()
    }

    fn initialise_at_root(
        &mut self,
        _: &mut PropagatorInitialisationContext,
    ) -> Result<(), PropositionalConjunction> {
        todo!()
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
