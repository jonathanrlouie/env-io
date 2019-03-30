# Env-IO

## What is Env-IO?

Env-IO is a highly experimental, work-in-progress, functional effect system for Rust that is inspired by the [ZIO library](https://github.com/scalaz/scalaz-zio) for Scala.
Specifically, it is based on ZIO environments, as described in [this article](http://degoes.net/articles/zio-environment).

## What is the point of this?

I wrote this for fun and because I wanted to learn more about Rust's traits. It also allows me experiment with the
various properties of functional effect systems in Rust.

## Usage with the mdo crate

The following code snippet shows an example of how to use Env-IO with the [mdo crate](https://github.com/TeXitoi/rust-mdo).

```rust
use mdo::mdo;
use env_io::{FlatMap, eff, EnvIO, IntoEnvIO};

fn bind<Env, OutEnv, F>(envio: Env, f: F) -> FlatMap<Env, F>
    where Env: EnvIO, OutEnv: EnvIO, F: Fn(Env::Out) -> OutEnv,
{
    envio.flat_map(f)
}

let program = mdo! {
    _ =<< eff!(println!("Enter your name: "));
    name =<< eff!({
        let mut name = String::new();
        io::stdin().read_line(&mut name).expect("Failed to read name.");
        name
    });
    ret eff!(println!("Hello, {}", name))
};
program.run();
```