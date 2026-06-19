use crate::game::types::Shape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEffect {
    HoldOn,
    PickTwo,
    PickThree,
    Suspension,
    GeneralMarket,
    Whot { called_shape: Shape },
}

pub fn suit_card_effect(value: u8) -> Option<ActionEffect> {
    match value {
        1  => Some(ActionEffect::HoldOn),
        2  => Some(ActionEffect::PickTwo),
        5  => Some(ActionEffect::PickThree),
        8  => Some(ActionEffect::Suspension),
        14 => Some(ActionEffect::GeneralMarket),
        _  => None,
    }
}
