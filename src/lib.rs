pub trait EnvIO {
    type Out;

    fn run(self) -> Self::Out;

    fn map<Out, F>(self, f: F) -> Map<Self, F>
        where Self: Sized, F: Fn(Self::Out) -> Out {
        Map { envio: self, f }
    }

    fn flat_map<OutEnv, F>(self, f: F) -> FlatMap<Self, F>
        where Self: Sized, OutEnv: EnvIO, F: Fn(Self::Out) -> OutEnv {
        FlatMap { envio: self, f }
    }

    fn flatten(self) -> FlatMap<Self, fn(Self::Out) -> Self::Out>
        where Self: Sized {
        FlatMap { envio: self, f: move |x| x }
    }
}

pub struct Effect<Out, F: Fn() -> Out> {
    effect: F
}

impl<Out0, F: Fn() -> Out0> EnvIO for Effect<Out0, F> {
    type Out = Out0;

    fn run(self) -> Self::Out {
        (self.effect)()
    }
}

pub struct Map<Env, F> {
    envio: Env,
    f: F
}

impl<Env: EnvIO, Out, F> EnvIO for Map<Env, F> where F: Fn(Env::Out) -> Out {
    type Out = Out;

    fn run(self) -> Out {
        (self.f)(self.envio.run())
    }
}

pub struct FlatMap<Env, F> {
    envio: Env,
    f: F
}

impl<Env: EnvIO, OutEnv: EnvIO, F> EnvIO for FlatMap<Env, F> where F: Fn(Env::Out) -> OutEnv {
    type Out = OutEnv::Out;

    fn run(self) -> OutEnv::Out {
        (self.f)(self.envio.run()).run()
    }
}

pub trait IntoEnvIO {
    type Out;

    type IntoEnv: EnvIO<Out=Self::Out>;

    fn effect(self) -> Self::IntoEnv;
}

impl<T, F: Fn() -> T> IntoEnvIO for F {
    type Out = T;
    type IntoEnv = Effect<T, F>;

    fn effect(self) -> Self::IntoEnv {
        Effect { effect: self }
    }
}

#[macro_export]
macro_rules! eff {
    ($e:expr) => {
        (move || $e).effect()
    }
}

#[cfg(test)]
mod tests {
    use mdo::mdo;
    use super::*;

    fn bind<Env, OutEnv, F>(envio: Env, f: F) -> FlatMap<Env, F>
        where Env: EnvIO, OutEnv: EnvIO, F: Fn(Env::Out) -> OutEnv,
    {
        envio.flat_map(f)
    }

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
    }
}

