use crate::{interpreter::Instr, BAny};

pub type Kleisli = Box<dyn Fn(BAny) -> Box<Instr>>;

pub enum KleisliOrFold {
    Kleisli(Kleisli),
    Fold(Kleisli, Kleisli),
}

impl KleisliOrFold {
    pub fn k(self) -> Kleisli {
        match self {
            KleisliOrFold::Kleisli(k) => k,
            KleisliOrFold::Fold(success, _failure) => success,
        }
    }
}
