pub trait EnvIO {
    type Out;

    fn run(self) -> Self::Out;

    fn flat_map<OutEnv, F>(self, f: F) -> FlatMap<Self, F>
    where Self: Sized, OutEnv: EnvIO, F: Fn(Self::Out) -> OutEnv {
        FlatMap { envio: self, f }
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
