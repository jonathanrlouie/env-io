use std::any::Any;
use std::marker::PhantomData;

type BAny = Box<dyn Any>;

fn downcast<T: 'static>(bany: BAny) -> T {
    *bany.downcast::<T>().unwrap()
}

type Kleisli = Box<dyn Fn(BAny) -> Box<Instr>>;

enum KleisliOrFold {
    Kleisli(Kleisli),
    Fold(Kleisli, Kleisli),
}

impl KleisliOrFold {
    fn k(self) -> Kleisli {
        match self {
            KleisliOrFold::Kleisli(k) => k,
            KleisliOrFold::Fold(success, _failure) => success,
        }
    }
}

pub enum NoReq {}
pub enum Nothing {}

enum Instr {
    Succeed(BAny),
    Fail(BAny),
    Effect(Box<dyn Fn() -> BAny>),
    FlatMap(Box<Instr>, KleisliOrFold),
    Fold(Box<Instr>, KleisliOrFold),
    Read(KleisliOrFold),
    Provide(BAny, Box<Instr>),
}

pub type UIO<A> = EnvIO<NoReq, A, Nothing>;
pub type URIO<R, A> = EnvIO<R, A, Nothing>;
pub type IO<A, E> = EnvIO<NoReq, A, E>;

pub struct ReqEnvIO<R, A, E> {
    envio: EnvIO<R, A, E>
}

impl<R: 'static, A: 'static, E: 'static> ReqEnvIO<R, A, E> {
    pub fn flat_map<B, K: Fn(A) -> EnvIO<NoReq, B, E> + 'static>(self, k: K) -> ReqEnvIO<R, B, E> {
        ReqEnvIO { envio: self.envio.flat_map(k) }
    }

    pub fn map<B: 'static, F: Fn(A) -> B + 'static>(self, f: F) -> ReqEnvIO<R, B, E> {
        ReqEnvIO { envio: self.envio.map(f) }
    }

    pub fn fold<S: 'static, F: 'static, B: 'static>(
        self,
        success: S,
        failure: F,
    ) -> ReqEnvIO<R, B, Nothing>
        where
            S: Fn(A) -> B,
            F: Fn(E) -> B,
    {
        ReqEnvIO { envio: self.envio.fold(success, failure) }
    }

    pub fn provide(self, r: R) -> IO<A, E> {
        provide(r)(self.envio)
    }
}

pub struct EnvIO<R, A, E> {
    instr: Instr,
    _pd: PhantomData<(R, A, E)>,
}

impl<R: 'static, A: 'static, E: 'static> EnvIO<R, A, E> {
    pub fn flat_map<B, K: Fn(A) -> EnvIO<NoReq, B, E> + 'static>(self, k: K) -> EnvIO<R, B, E> {
        EnvIO {
            instr: Instr::FlatMap(
                box_instr(self),
                KleisliOrFold::Kleisli(Box::new(move |bany: BAny| {
                    box_instr(k(downcast::<A>(bany)))
                })),
            ),
            _pd: PhantomData,
        }
    }

    pub fn map<B: 'static, F: Fn(A) -> B + 'static>(self, f: F) -> EnvIO<R, B, E> {
        EnvIO {
            instr: Instr::FlatMap(
                box_instr(self),
                KleisliOrFold::Kleisli(Box::new(move |bany: BAny| {
                    box_instr(succeed(f(downcast::<A>(bany))))
                })),
            ),
            _pd: PhantomData,
        }
    }

    pub fn fold<S: 'static, F: 'static, B: 'static>(
        self,
        success: S,
        failure: F,
    ) -> EnvIO<R, B, Nothing>
    where
        S: Fn(A) -> B,
        F: Fn(E) -> B,
    {
        EnvIO {
            instr: Instr::Fold(
                box_instr(self),
                KleisliOrFold::Fold(
                    Box::new(move |bany: BAny| box_instr(succeed(success(downcast::<A>(bany))))),
                    Box::new(move |bany: BAny| box_instr(succeed(failure(downcast::<E>(bany))))),
                ),
            ),
            _pd: PhantomData,
        }
    }
}

impl<A: 'static> UIO<A> {
    pub fn into_envio<R: 'static, E: 'static>(self) -> EnvIO<R, A, E> {
        EnvIO {
            instr: self.instr,
            _pd: PhantomData,
        }
    }
}

impl<A: 'static, E: 'static> IO<A, E> {
    pub fn with_env<R: 'static>(self) -> EnvIO<R, A, E> {
        EnvIO {
            instr: self.instr,
            _pd: PhantomData,
        }
    }

    pub fn flat_map_req<R: 'static, B, K: Fn(A) -> ReqEnvIO<R, B, E> + 'static>(
        self,
        k: K,
    ) -> ReqEnvIO<R, B, E> {
        ReqEnvIO {
            envio: EnvIO {
                instr: Instr::FlatMap(
                    box_instr(self),
                    KleisliOrFold::Kleisli(Box::new(move |bany: BAny| {
                        box_instr(k(downcast::<A>(bany)).envio)
                    })),
                ),
                _pd: PhantomData,
            }
        }
    }
}

fn box_instr<R, E, A>(envio: EnvIO<R, E, A>) -> Box<Instr> {
    Box::new(envio.instr)
}

fn provide<R: 'static, A: 'static, E: 'static>(r: R) -> impl FnOnce(EnvIO<R, A, E>) -> IO<A, E> {
    move |envio: EnvIO<R, A, E>| EnvIO {
        instr: Instr::Provide(Box::new(r), box_instr(envio)),
        _pd: PhantomData,
    }
}

pub fn environment<R: 'static>() -> ReqEnvIO<R, R, Nothing> {
    ReqEnvIO {
        envio: EnvIO {
            instr: Instr::Read(KleisliOrFold::Kleisli(Box::new(move |bany: BAny| {
                box_instr(succeed(downcast::<R>(bany)))
            }))),
            _pd: PhantomData,
        }
    }
}

#[macro_export]
macro_rules! effect {
    ($e:expr) => {{
        $crate::effect(move || $e)
    }};
}

pub fn effect<A: 'static, F: 'static>(eff: F) -> UIO<A>
where
    F: Fn() -> A,
{
    // Cannot inline, since the compiler is not able to automatically infer the output type as Box<dyn Any>
    let effect_any = move || {
        let bany: BAny = Box::new(eff());
        bany
    };

    EnvIO {
        instr: Instr::Effect(Box::new(effect_any)),
        _pd: PhantomData,
    }
}

pub fn succeed<A: 'static>(a: A) -> UIO<A> {
    EnvIO {
        instr: Instr::Succeed(Box::new(a)),
        _pd: PhantomData,
    }
}

pub fn fail<E: 'static>(e: E) -> IO<Nothing, E> {
    EnvIO {
        instr: Instr::Fail(Box::new(e)),
        _pd: PhantomData,
    }
}

pub fn run(envio: EnvIO<NoReq, (), Nothing>) {
    interpret::<(), Nothing>(envio.instr).unwrap_or(());
}

pub fn run_result<A: 'static, E: 'static>(envio: EnvIO<NoReq, A, E>) -> Result<A, E> {
    interpret::<A, E>(envio.instr)
}

fn interpret<A: 'static, E: 'static>(mut instr: Instr) -> Result<A, E> {
    let mut stack: Vec<KleisliOrFold> = vec![];
    let mut environments: Vec<BAny> = vec![];

    loop {
        match instr {
            Instr::FlatMap(inner, kleisli) => match *inner {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mdo::mdo;

    #[test]
    fn test() {
        let i1 = succeed(3u32);
        let i2 =
            i1.flat_map(move |a| succeed(5u32).flat_map(move |b| effect!(println!("{}", a + b))));

        let result: () = match run_result(i2) {
            Ok(a) => a,
            Err(_) => unimplemented!(),
        };
        assert_eq!(result, ());
    }

    #[test]
    fn test_fail() {
        let i1: IO<u32, u32> = succeed(3u32).into_envio();
        let i2: IO<(), u32> = i1.flat_map(move |a| {
            fail(5u32).flat_map(move |b| effect!(println!("test")).into_envio())
        });

        let result: u32 = match run_result(i2) {
            Ok(_) => unimplemented!(),
            Err(e) => e,
        };
        assert_eq!(result, 5);
    }

    #[test]
    fn test_fold() {
        let i1: IO<u32, u32> = succeed(3u32).into_envio();
        let i2: IO<(), u32> = i1.flat_map(move |a| {
            fail(5u32).flat_map(move |b| effect!(println!("test")).into_envio())
        });

        let i3 = i2.fold(|u| "success".to_string(), |u32| "fail".to_string());

        let result: String = match run_result(i3) {
            Ok(s) => s,
            Err(_) => unimplemented!(),
        };
        assert_eq!(result, "fail".to_string());
    }

    #[test]
    fn test_map() {
        let i1: UIO<u32> = succeed(3u32).into_envio();
        let i2 = i1.map(|u: u32| u > 2);
        let result = match run_result(i2) {
            Ok(b) => b,
            Err(_) => unimplemented!(),
        };
        assert_eq!(result, true);
    }

    #[test]
    fn test_environment() {
        let envio: ReqEnvIO<u32, u32, Nothing> = environment::<u32>().map(move |env| env * env);
        let next = envio.provide(4);
        let result = match run_result(next) {
            Ok(int) => int,
            Err(_) => unimplemented!(),
        };
        assert_eq!(result, 16)
    }

    #[test]
    fn test_environment_add_req() {
        let uio: UIO<u32> = succeed(2u32);
        let envio = uio.flat_map_req(move |value| {
            environment::<u32>().flat_map(move |env| succeed(env * value))
        });

        let result = match run_result(envio.provide(4)) {
            Ok(int) => int,
            Err(_) => unimplemented!(),
        };
        assert_eq!(result, 8)
    }

    /* Try to find way to test that this doesn't compile
    #[test]
    fn test_environment_no_req() {
        let result = match run_result(environment::<NoReq>()) {
            Ok(int) => int,
            Err(_) => unimplemented!(),
        };
    }*/
}
