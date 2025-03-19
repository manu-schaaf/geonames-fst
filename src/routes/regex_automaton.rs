use std::str::FromStr;

use regex_automata::dfa::dense::DFA;
use regex_automata::dfa::{dense, Automaton as RegexAutomaton};
use regex_automata::util::primitives::StateID;
use regex_automata::Input;

#[derive(Debug)]
pub(crate) struct RegexSearchAutomaton {
    dfa: DFA<Vec<u32>>,
    start_state: StateID,
}

impl FromStr for RegexSearchAutomaton {
    type Err = anyhow::Error;

    fn from_str(query: &str) -> Result<Self, Self::Err> {
        let dfa = dense::DFA::new(query)?;
        let start_state = dfa.start_state_forward(&Input::new(query))?;
        Ok(RegexSearchAutomaton { dfa, start_state })
    }
}

impl fst::Automaton for RegexSearchAutomaton {
    type State = Option<StateID>;

    #[inline]
    fn start(&self) -> Option<StateID> {
        Some(self.start_state)
    }

    fn is_match(&self, state: &Self::State) -> bool {
        state
            .map(|state| self.dfa.is_match_state(self.dfa.next_eoi_state(state)))
            .unwrap_or(false)
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        state.and_then(|state| Some(self.dfa.next_state(state, byte)))
    }
}
