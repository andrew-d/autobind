#![feature(phase)]
#![allow(dead_code)]

#[phase(plugin)] extern crate autobind;

struct Foo;

#[autobind]
impl Foo {
    pub fn with_lifetime<'a>(&'a self) {}
    //~^ WARNING: autobind: cannot generate bindings for method with a lifetime on `self`

    pub fn by_value(self) {}
    //~^ WARNING: autobind: cannot generate bindings for method with by-value `self`

    pub fn explicit_self(self: Box<Foo>) {}
    //~^ WARNING: autobind: cannot generate bindings for method with explicit `self`

    // No warning for items that aren't public.
    fn not_public_with_lifetime<'a>(&'a self) {}
}

// At least one error is needed so that compilation fails
#[static_assert]
static b: bool = false; //~ ERROR static assertion failed

fn main() {}
