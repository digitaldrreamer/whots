use crate::game::ai::types::{Candidate, ModuleContext};

/// Bias toward action cards. 5 (pick three) valued most; scores dominate hand-thinning.
pub fn action_awareness(candidate: &Candidate, _ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlaySuit { value, .. } = candidate else {
        return 0.0;
    };
    match value {
        5 => 30.0,  // pick three
        2 => 25.0,  // pick two
        14 => 20.0, // general market
        1 => 15.0,  // hold on
        8 => 15.0,  // suspension
        _ => 0.0,
    }
}
