use std::any::Any;
use std::marker::PhantomData;

type BAny = Box<dyn Any>;

type Kleisli = Box<Fn(BAny) -> Box<Instr>>;

enum Instr {
    Succeed(BAny),
    Effect(Box<Fn() -> BAny>),
    FlatMap(Box<Instr>, Kleisli),
}

struct EnvIO<R, A, E> {
    instr: Instr,
    _pd: PhantomData<(R, A, E)>,
}

impl<R: 'static, A: 'static, E: 'static> EnvIO<R, A, E> {
    fn flat_map<B, K: Fn(A) -> EnvIO<R, B, E> + 'static>(self, k: K) -> EnvIO<R, B, E> {
        let any_input_k = move |bany: BAny| {
            let a: Box<A> = bany
                .downcast::<A>()
                .unwrap_or_else(|_| panic!("flat_map: Could not downcast Any to A"));
            k(*a)
        };

        let instr_output_k = move |bany: BAny| Box::new(any_input_k(bany).instr);

        EnvIO {
            instr: Instr::FlatMap(Box::new(self.instr), Box::new(instr_output_k)),
            _pd: PhantomData,
        }
    }
}

macro_rules! effect {
    ($e:expr) => {{
        $crate::effect(move || $e)
    }};
}

fn effect<R, A: 'static, E, F: 'static>(eff: F) -> EnvIO<R, A, E>
where
    F: Fn() -> A,
{
    let effect_any = move || {
        let bany: BAny = Box::new(eff());
        bany
    };

    EnvIO {
        instr: Instr::Effect(Box::new(effect_any)),
        _pd: PhantomData,
    }
}

fn success<R, A: 'static, E>(a: A) -> EnvIO<R, A, E> {
    EnvIO {
        instr: Instr::Succeed(Box::new(a)),
        _pd: PhantomData,
    }
}

fn run<R, A: 'static, E>(envio: EnvIO<R, A, E>) -> A {
    interpret::<A>(envio.instr)
}

fn interpret<A: 'static>(mut instr: Instr) -> A {
    let mut stack: Vec<Kleisli> = vec![];
    loop {
        match instr {
            Instr::FlatMap(inner, kleisli) => match *inner {
                Instr::Effect(eff) => instr = *kleisli(eff()),
                Instr::Succeed(a) => {
                    instr = *kleisli(a);
                }
                _ => {
                    stack.push(kleisli);
                    instr = *inner;
                }
            },
            Instr::Effect(eff) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli(eff());
                } else {
                    return *eff().downcast::<A>().unwrap_or_else(|_| {
                        panic!("interpret (effect): Could not downcast Any to A")
                    });
                }
            }
            Instr::Succeed(a) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli(a);
                } else {
                    return *a.downcast::<A>().unwrap_or_else(|_| {
                        panic!("interpret (succeed): Could not downcast Any to A")
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdo::mdo;

    #[test]
    fn test() {
        let i1: EnvIO<(), u32, ()> = success(3u32);
        let i2 =
            i1.flat_map(move |a| success(5u32).flat_map(move |b| effect!(println!("{}", a + b))));

        assert_eq!(run::<(), (), ()>(i2), ());
    }
}
