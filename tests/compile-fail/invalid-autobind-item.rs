#![feature(phase)]

#[phase(plugin)] extern crate autobind;

#[autobind]  //~ ERROR: #[autobind] can only be applied to impl blocks
fn not_an_impl() { }

struct Foo;

#[autobind] //~ ERROR: #[autobind] cannot be applied to impls of traits
impl PartialEq for Foo {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}

fn main() {
    not_an_impl()
}
