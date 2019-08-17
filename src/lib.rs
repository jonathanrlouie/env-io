use std::any::Any;

pub mod envio;
mod interpreter;
mod kleisli;

type BAny = Box<dyn Any>;

fn downcast<T: 'static>(bany: BAny) -> T {
    // unwrap here because any failure to downcast means there is a bug in this library
    *bany.downcast::<T>().unwrap()
}
