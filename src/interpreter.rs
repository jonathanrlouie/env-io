use crate::{downcast, kleisli::KleisliOrFold, BAny};

pub enum Instr {
    Succeed(BAny),
    Fail(BAny),
    Effect(Box<dyn Fn() -> BAny>),
    AndThen(Box<Instr>, KleisliOrFold),
    Fold(Box<Instr>, KleisliOrFold),
    Read(KleisliOrFold),
    Provide(BAny, Box<Instr>),
}

pub fn interpret<A: 'static, E: 'static>(mut instr: Instr) -> Result<A, E> {
    let mut stack: Vec<KleisliOrFold> = vec![];
    let mut environments: Vec<BAny> = vec![];

    loop {
        match instr {
            Instr::AndThen(inner, kleisli) => match *inner {
                Instr::Effect(eff) => instr = *kleisli.k()(eff()),
                Instr::Succeed(a) => {
                    instr = *kleisli.k()(a);
                }
                _ => {
                    stack.push(kleisli);
                    instr = *inner;
                }
            },
            Instr::Effect(eff) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli.k()(eff());
                } else {
                    return Ok(downcast::<A>(eff()));
                }
            }
            Instr::Succeed(a) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli.k()(a);
                } else {
                    return Ok(downcast::<A>(a));
                }
            }
            Instr::Fold(inner, kleisli) => {
                stack.push(kleisli);
                instr = *inner;
            }
            Instr::Fail(e) => {
                unwind_stack(&mut stack);
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli.k()(e);
                } else {
                    return Err(downcast::<E>(e));
                }
            }
            Instr::Read(kleisli) => {
                if let Some(env) = environments.pop() {
                    instr = *kleisli.k()(env);
                } else {
                    panic!("No environments on the stack");
                }
            }
            Instr::Provide(r, next) => {
                environments.push(r);
                instr = *next;
            }
        }
    }
}

fn unwind_stack(stack: &mut Vec<KleisliOrFold>) {
    while let Some(kleisli_or_fold) = stack.pop() {
        match kleisli_or_fold {
            KleisliOrFold::Fold(_success, failure) => {
                stack.push(KleisliOrFold::Kleisli(failure));
                break;
            }
            KleisliOrFold::Kleisli(_k) => (),
        }
    }
}
