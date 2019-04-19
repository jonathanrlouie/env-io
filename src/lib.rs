use std::marker::PhantomData;

pub trait EnvIO {
    type In;
    type Out;

    fn run(self, input: &Self::In) -> Self::Out;

    fn flat_map<OutEnv, F>(self, f: F) -> FlatMap<Self, F>
        where Self: Sized, OutEnv: EnvIO, F: Fn(Self::Out) -> OutEnv {
        FlatMap { envio: self, f }
    }

    fn map<Out, F>(self, f: F) -> Map<Self, F>
        where Self: Sized, F: Fn(Self::Out) -> Out {
        Map { envio: self, f }
    }

    fn flatten(self) -> FlatMap<Self, fn(Self::Out) -> Self::Out>
        where Self: Sized {
        FlatMap { envio: self, f: move |x| x }
    }

}

pub struct Effect<In, Out, F: Fn(&In) -> Out> {
    effect: F,
    _pd: PhantomData<In>
}

impl<In, Out0, F: Fn(&In) -> Out0> EnvIO for Effect<In, Out0, F> {
    type In = In;
    type Out = Out0;

    fn run(self, input: &Self::In) -> Self::Out {
        (self.effect)(input)
    }
}


pub struct Map<Env, F> {
    envio: Env,
    f: F
}

impl<Env: EnvIO, Out, F> EnvIO for Map<Env, F> where F: Fn(Env::Out) -> Out {
    type In = Env::In;
    type Out = Out;

    fn run(self, input: &Self::In) -> Out {
        (self.f)(self.envio.run(input))
    }
}



pub struct FlatMap<Env, F> {
    envio: Env,
    f: F
}

impl<Env: EnvIO, OutEnv: EnvIO<In=Env::In>, F> EnvIO for FlatMap<Env, F> where F: Fn(Env::Out) -> OutEnv {
    type In = Env::In;
    type Out = OutEnv::Out;

    fn run(self, input: &Self::In) -> OutEnv::Out {
        (self.f)(self.envio.run(input)).run(input)
    }
}

pub trait IntoEnvIO<In> {
    type Out;

    type IntoEnv: EnvIO<In=In, Out=Self::Out>;

    fn effect(self) -> Self::IntoEnv;
}

impl<In, T, F: Fn(&In) -> T> IntoEnvIO<In> for F {
    type Out = T;
    type IntoEnv = Effect<In, T, F>;

    fn effect(self) -> Self::IntoEnv {
        Effect { effect: self, _pd: PhantomData }
    }
}

pub fn eff<A, B, F: Fn(&A) -> B>(f: F) -> Effect<A, B, F> {
    f.effect()
}

/*
#[macro_export]
macro_rules! eff {
    ($t:ty, $e:expr) => {
        (move |_: &$t| $e).effect()
    }
}*/

#[cfg(test)]
mod tests {
    use mdo::mdo;
    use super::*;

    /*
    fn bind<Env, OutEnv, F>(envio: Env, f: F) -> FlatMap<Env, F>
        where Env: EnvIO, OutEnv: EnvIO, F: Fn(Env::Out) -> OutEnv,
    {
        envio.flat_map(f)
    }*/

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
        //println!("hi");
        //let e = eff(|a: &i32| println!("hi {}", a.to_string()));
        //let e = eff!(i32, "hi");

        /*
        let e = mdo! {
            _ =<< println("Hi world");
            ret println("yo")
        };*/

        let e = println("hi".to_string())
            .flat_map(|_| println("yo".to_string()));

        let console_mod = ConsoleModule {
            console: TestConsole
        };
        assert_eq!(e.run(&console_mod), "yo");
    }

    /*
    #[test]
    fn test_flatten() {
        let e = mdo! {
            x =<< eff!(eff!("hi")).flatten();
            ret eff!(x)
        };
        assert_eq!(e.run(), "hi");
    }

    #[test]
    fn test_map() {
        let e = mdo! {
            x =<< eff!(2u32)
                .map(|i: u32| 2 * i)
                .map(|i: u32| i as i32);
            ret eff!(x)
        };
        assert_eq!(e.run(), 4);
    }*/
}

