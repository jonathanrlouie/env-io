use std::marker::PhantomData;
use std::mem;

struct Kleisli<R, A, E> {
    k: Box<dyn Fn(Box<Opaque>) -> EnvIO<Opaque, Opaque, Opaque>>,
    _pd: PhantomData<(R, A, E)>
}

enum Opaque {}

enum EnvIO<R, A, E> {
    Effect(Box<dyn Fn() -> Box<A>>),
    Succeed(Box<A>),
    Fail(Box<E>),
    Provide(Box<R>),
    FlatMap(Box<EnvIO<R, Opaque, E>>, Kleisli<R, A, E>)
}

impl<R, A, E> EnvIO<R, A, E> {
    unsafe fn flat_map_unsafe<B>(self, k: Box<dyn Fn(Box<A>) -> EnvIO<R, B, E>>) -> EnvIO<R, B, E> {
        EnvIO::FlatMap(Box::new(mem::transmute(self)), Kleisli { k: mem::transmute(k), _pd: PhantomData })
    }

    fn flat_map<B, F: 'static>(self, k: F) -> EnvIO<R, B, E> where F: Fn(A) -> EnvIO<R, B, E> {
        unsafe { self.flat_map_unsafe(Box::new(move |a: Box<A>| k(*a))) }
    }
}

macro_rules! effect {
    ($e:expr) => {
        {
			$crate::effect(move || Box::new($e))
        }
    }
}

fn effect<R, A, E, F: 'static>(eff: F) -> EnvIO<R, A, E> where F: Fn() -> Box<A> {
    EnvIO::Effect(Box::new(eff))
}

fn succeed<R, A, E>(a: A) -> EnvIO<R, A, E> {
    EnvIO::Succeed(Box::new(a))
}

fn run<R, A, E>(envio: EnvIO<R, A, E>) -> A {
    unsafe {
        let envio_opaque: EnvIO<Opaque, Opaque, Opaque> = mem::transmute(envio);
        interpret(envio_opaque)
    }
}

unsafe fn interpret<A>(mut envio: EnvIO<Opaque, Opaque, Opaque>) -> A {
    let mut stack: Vec<Kleisli<Opaque, Opaque, Opaque>> = vec![];
    loop {
        match envio {
            EnvIO::FlatMap(inner, kleisli) => {
                match *inner {
                    EnvIO::Effect(eff) => {
                        envio = (kleisli.k)(eff())
                    }
                    EnvIO::Succeed(a) => {
                        envio = (kleisli.k)(a);
                    }
                    _ => {
                        stack.push(kleisli);
                        envio = *inner;
                    }
                }
            },
            EnvIO::Effect(eff) => {
                if let Some(kleisli) = stack.pop() {
                    envio = (kleisli.k)(eff());
                } else {
                    return *mem::transmute::<Box<Opaque>, Box<A>>(eff());
                }
            }
            EnvIO::Succeed(a) => {
                if let Some(kleisli) = stack.pop() {
                    envio = (kleisli.k)(a);
                } else {
                    return *mem::transmute::<Box<Opaque>, Box<A>>(a);
                }
            },
            _ => panic!("unknown instruction")
        }
    }
}

#[cfg(test)]
mod tests {
    use mdo::mdo;
    use super::*;

    #[test]
    fn test() {
        println!("hi")
    }
    /*
    trait Console<T> {
        fn println(&self, line: &str) -> T;
    }

    struct ProductionConsole;

    struct ConsoleModule<T> {
        console: T
    }

    impl Console<()> for ProductionConsole {
        fn println(&self, line: &str) {
            println!("{}", line)
        }
    }

    struct TestConsole;

    impl Console<String> for TestConsole {
        fn println(&self, line: &str) -> String {
            line.to_string()
        }
    }

    fn println<A, T: Console<A>>(line: String) -> impl EnvIO<In=ConsoleModule<T>, Out=A> {
        eff(move |console_mod: &ConsoleModule<T>| console_mod.console.println(&line))
    }

    #[test]
    fn test() {
        let e = mdo! {
            _ =<< println("Hi world".to_string());
            ret println("yo".to_string())
        };

        let console_mod = ConsoleModule {
            console: TestConsole
        };
        assert_eq!(e.run(&console_mod), "yo");
    }

    #[test]
    fn test_flatten() {
        let e = mdo! {
            x =<< eff!(eff!("hi")).flatten();
            ret eff!(x)
        };
        assert_eq!(e.run(&()), "hi");
    }

    #[test]
    fn test_map() {
        let e = mdo! {
            x =<< eff!(2u32)
                .map(|i: u32| 2 * i)
                .map(|i: u32| i as i32);
            ret eff!(x)
        };
        assert_eq!(e.run(&()), 4);
    }*/
}

