//! Portable deterministic scan primitives.
//!
//! This crate is the lowest execution layer in Aaroneous. It deliberately has
//! no allocator, operating-system, runtime, serialization, or device
//! dependency. Platform crates acquire inputs and apply outputs; reducers only
//! calculate the next bounded state and output.

#![no_std]
#![deny(unsafe_code)]
#![deny(missing_docs)]

use core::marker::PhantomData;

/// A deterministic state transition executed during the reduction phase of a
/// scan cycle.
///
/// Implementations receive committed values and pre-populated scratch values.
/// They may update the scratch values in place. Returning `Ok(())` commits both
/// scratch values atomically from the machine's perspective; returning an error
/// leaves the previously committed state and output visible.
///
/// Acquisition, actuation, clocks, files, networking, allocation, and device
/// access belong outside this trait.
pub trait ScanReducer {
    /// Fixed-memory state retained between committed cycles.
    type State: Copy;

    /// Input frame acquired before reduction begins.
    type Input;

    /// Fixed-memory output frame applied after a successful reduction.
    type Output: Copy;

    /// Domain error returned when a transition cannot be committed.
    type Error;

    /// Calculates one transition without performing external side effects.
    fn reduce(
        current_state: &Self::State,
        input: &Self::Input,
        next_state: &mut Self::State,
        next_output: &mut Self::Output,
    ) -> Result<(), Self::Error>;
}

/// Double-buffered owner of one reducer's committed state and output.
///
/// Both buffers are constructed up front. A scan therefore performs no
/// allocation and cannot expose a partially committed transition.
pub struct ScanMachine<R>
where
    R: ScanReducer,
{
    committed_state: R::State,
    next_state: R::State,
    committed_output: R::Output,
    next_output: R::Output,
    committed_cycles: u64,
    reducer: PhantomData<fn() -> R>,
}

impl<R> ScanMachine<R>
where
    R: ScanReducer,
{
    /// Creates a machine with caller-supplied fixed-memory buffers.
    #[must_use]
    pub const fn new(initial_state: R::State, initial_output: R::Output) -> Self {
        Self {
            committed_state: initial_state,
            next_state: initial_state,
            committed_output: initial_output,
            next_output: initial_output,
            committed_cycles: 0,
            reducer: PhantomData,
        }
    }

    /// Executes one reduction and commits it only when the reducer succeeds.
    ///
    /// The returned value is the total number of successful committed cycles.
    pub fn tick(&mut self, input: &R::Input) -> Result<u64, R::Error> {
        self.next_state = self.committed_state;
        self.next_output = self.committed_output;

        if let Err(error) = R::reduce(
            &self.committed_state,
            input,
            &mut self.next_state,
            &mut self.next_output,
        ) {
            self.next_state = self.committed_state;
            self.next_output = self.committed_output;
            return Err(error);
        }

        core::mem::swap(&mut self.committed_state, &mut self.next_state);
        core::mem::swap(&mut self.committed_output, &mut self.next_output);
        self.committed_cycles = self.committed_cycles.saturating_add(1);
        Ok(self.committed_cycles)
    }

    /// Returns the last successfully committed state.
    #[must_use]
    pub const fn state(&self) -> &R::State {
        &self.committed_state
    }

    /// Returns the output associated with the last successfully committed
    /// state.
    #[must_use]
    pub const fn output(&self) -> &R::Output {
        &self.committed_output
    }

    /// Returns the number of successfully committed scan cycles.
    #[must_use]
    pub const fn committed_cycles(&self) -> u64 {
        self.committed_cycles
    }

    /// Replaces both buffers with explicit caller-supplied values and resets
    /// the committed-cycle counter.
    pub fn reset(&mut self, state: R::State, output: R::Output) {
        self.committed_state = state;
        self.next_state = state;
        self.committed_output = output;
        self.next_output = output;
        self.committed_cycles = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{ScanMachine, ScanReducer};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct State {
        total: i32,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Output {
        doubled: i32,
    }

    #[derive(Clone, Copy)]
    struct Input {
        delta: i32,
        reject: bool,
    }

    struct Accumulator;

    impl ScanReducer for Accumulator {
        type State = State;
        type Input = Input;
        type Output = Output;
        type Error = ();

        fn reduce(
            current_state: &Self::State,
            input: &Self::Input,
            next_state: &mut Self::State,
            next_output: &mut Self::Output,
        ) -> Result<(), Self::Error> {
            next_state.total = current_state.total + input.delta;
            next_output.doubled = next_state.total * 2;
            if input.reject {
                return Err(());
            }
            Ok(())
        }
    }

    fn machine() -> ScanMachine<Accumulator> {
        ScanMachine::new(State { total: 4 }, Output { doubled: 8 })
    }

    #[test]
    fn identical_inputs_replay_identically() {
        let inputs = [
            Input {
                delta: 3,
                reject: false,
            },
            Input {
                delta: -1,
                reject: false,
            },
            Input {
                delta: 8,
                reject: false,
            },
        ];
        let mut first = machine();
        let mut second = machine();

        for input in &inputs {
            assert_eq!(first.tick(input), second.tick(input));
        }

        assert_eq!(first.state(), second.state());
        assert_eq!(first.output(), second.output());
        assert_eq!(first.committed_cycles(), 3);
    }

    #[test]
    fn rejected_transition_commits_neither_state_nor_output() {
        let mut machine = machine();
        let before_state = *machine.state();
        let before_output = *machine.output();

        assert_eq!(
            machine.tick(&Input {
                delta: 100,
                reject: true,
            }),
            Err(())
        );
        assert_eq!(*machine.state(), before_state);
        assert_eq!(*machine.output(), before_output);
        assert_eq!(machine.committed_cycles(), 0);
    }

    #[test]
    fn reset_replaces_both_buffers_and_counter() {
        let mut machine = machine();
        assert_eq!(
            machine.tick(&Input {
                delta: 2,
                reject: false,
            }),
            Ok(1)
        );

        machine.reset(State { total: 9 }, Output { doubled: 18 });

        assert_eq!(*machine.state(), State { total: 9 });
        assert_eq!(*machine.output(), Output { doubled: 18 });
        assert_eq!(machine.committed_cycles(), 0);
    }
}
