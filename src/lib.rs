use std::any::Any;
use std::marker::PhantomData;

type BAny = Box<dyn Any>;

type Kleisli = Box<dyn Fn(BAny) -> Box<Instr>>;

enum KleisliOrFold{
    Kleisli(Kleisli),
    Fold(Kleisli, Kleisli)
}

impl KleisliOrFold {
    fn k(self) -> Kleisli {
        match self {
            KleisliOrFold::Kleisli(k) => k,
            KleisliOrFold::Fold(success, _failure) => success
        }
    }
}

type UIO<A> = EnvIO<BAny, A, Nothing>;
type IO<A, E> = EnvIO<BAny, A, E>;

enum Nothing {}

enum Instr {
    Succeed(BAny),
    Fail(BAny),
    Effect(Box<dyn Fn() -> BAny>),
    FlatMap(Box<Instr>, KleisliOrFold),
    Fold(Box<Instr>, KleisliOrFold)
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
                .expect("flat_map: Could not downcast Any to A");
            k(*a)
        };

        let instr_output_k = move |bany: BAny| Box::new(any_input_k(bany).instr);

        EnvIO {
            instr: Instr::FlatMap(Box::new(self.instr), KleisliOrFold::Kleisli(Box::new(instr_output_k))),
            _pd: PhantomData,
        }
    }

    fn map<B: 'static, F: Fn(A) -> B + 'static>(self, f: F) -> EnvIO<R, B, E> {
        let any_input_f = move |bany: BAny| {
            let a: Box<A> = bany
                .downcast::<A>()
                .expect("flat_map: Could not downcast Any to A");
            f(*a)
        };

        let instr_output_f = move |bany: BAny| Box::new(succeed(any_input_f(bany)).instr);

        EnvIO {
            instr: Instr::FlatMap(Box::new(self.instr), KleisliOrFold::Kleisli(Box::new(instr_output_f))),
            _pd: PhantomData
        }
    }

    fn fold<S: 'static, F: 'static, B: 'static>(self, success: S, failure: F) -> EnvIO<R, B, Nothing>
    where S: Fn(A) -> B, F: Fn(E) -> B {
        let any_input_success = move |bany: BAny| {
            let a: Box<A> = bany
                .downcast::<A>()
                .expect("fold: Could not downcast Any to A");
            success(*a)
        };

        let envio_output_success = move |bany: BAny| Box::new(succeed(any_input_success(bany)).instr);

        let any_input_failure = move |bany: BAny| {
            let e: Box<E> = bany
                .downcast::<E>()
                .expect("fold: Could not downcast Any to A");
            failure(*e)
        };

        let envio_output_failure = move |bany: BAny| Box::new(succeed(any_input_failure(bany)).instr);

        EnvIO {
            instr: Instr::Fold(
                Box::new(self.instr),
                KleisliOrFold::Fold(
                    Box::new(envio_output_success),
                    Box::new(envio_output_failure)
                )
            ),
            _pd: PhantomData
        }
    }
}

impl<A: 'static> UIO<A> {
    fn into_envio<R: 'static, E: 'static>(self) -> EnvIO<R, A, E> {
        EnvIO {
            instr: self.instr,
            _pd: PhantomData
        }
    }
}

impl<A: 'static, E: 'static> IO<A, E> {
    fn with_env<R: 'static>(self) -> EnvIO<R, A, E> {
        EnvIO {
            instr: self.instr,
            _pd: PhantomData
        }
    }
}

macro_rules! effect {
    ($e:expr) => {{
        $crate::effect(move || $e)
    }};
}

fn effect<A: 'static, F: 'static>(eff: F) -> UIO<A>
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

fn succeed<A: 'static>(a: A) -> UIO<A> {
    EnvIO {
        instr: Instr::Succeed(Box::new(a)),
        _pd: PhantomData,
    }
}

fn fail<E: 'static>(e: E) -> IO<Nothing, E> {
    EnvIO {
        instr: Instr::Fail(Box::new(e)),
        _pd: PhantomData,
    }
}



fn run<R, A: 'static, E: 'static>(envio: EnvIO<R, A, E>) -> Result<A, E> {
    interpret::<A, E>(envio.instr)
}

fn interpret<A: 'static, E: 'static>(mut instr: Instr) -> Result<A, E> {
    let mut stack: Vec<KleisliOrFold> = vec![];
    loop {
        match instr {
            Instr::FlatMap(inner, kleisli) => match *inner {
                Instr::Effect(eff) => {
                    instr = *kleisli.k()(eff())
                },
                Instr::Succeed(a) => {
                    instr = *kleisli.k()(a);
                },
                _ => {
                    stack.push(kleisli);
                    instr = *inner;
                }
            },
            Instr::Effect(eff) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli.k()(eff());
                } else {
                    return Ok(*eff()
                        .downcast::<A>()
                        .expect("interpret (effect): Could not downcast Any to A"));
                }
            }
            Instr::Succeed(a) => {
                if let Some(kleisli) = stack.pop() {
                    instr = *kleisli.k()(a);
                } else {
                    return Ok(*a
                        .downcast::<A>()
                        .expect("interpret (succeed): Could not downcast Any to A"));
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
                    return Err(*e
                        .downcast::<E>()
                        .expect("interpret (fail): Could not downcast Any to E"));
                }
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
            },
            KleisliOrFold::Kleisli(_k) => ()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdo::mdo;

    #[test]
    fn test() {
        let i1 = succeed(3u32);
        let i2 = i1
            .flat_map(move |a| succeed(5u32)
            .flat_map(move |b| effect!(println!("{}", a + b))));

        let result: () = match run(i2) {
            Ok(a) => a,
            Err(_) => unimplemented!()
        };
        assert_eq!(result, ());
    }

    #[test]
    fn test_fail() {
        let i1: EnvIO<BAny, u32, u32> = succeed(3u32).into_envio();
        let i2: EnvIO<BAny, (), u32> = i1
            .flat_map(move |a| fail(5u32)
                .flat_map(move |b| effect!(println!("test")).into_envio()));

        let result: u32 = match run(i2) {
            Ok(_) => unimplemented!(),
            Err(e) => e
        };
        assert_eq!(result, 5);
    }

    #[test]
    fn test_fold() {
        let i1: EnvIO<BAny, u32, u32> = succeed(3u32).into_envio();
        let i2: EnvIO<BAny, (), u32> = i1
            .flat_map(move |a| fail(5u32)
                .flat_map(move |b| effect!(println!("test")).into_envio()));

        let i3 = i2.fold(|u| "success".to_string(), |u32| "fail".to_string());

        let result: String = match run(i3) {
            Ok(s) => s,
            Err(e) => unimplemented!()
        };
        assert_eq!(result, "fail".to_string());
    }
}
