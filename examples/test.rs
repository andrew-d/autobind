#![feature(phase)]

#[phase(plugin)] extern crate autobind;

pub struct TestStruct {
    foo: int,
}

#[autobind]
impl TestStruct {
    pub fn new(val: int) -> TestStruct {
        TestStruct {
            foo: val,
        }
    }

    pub fn do_a_thing(&self) {
        println!("The value of 'foo' is {}", self.foo);
    }

    pub fn get_thing(&self) -> int {
        self.foo
    }

    pub fn with_lifetime<'a>(&'a self) {

    }

    pub fn by_value(self) -> int {
        self.foo
    }

    pub fn explicit_self(self: Box<TestStruct>) -> int {
        self.foo
    }
}

fn main() {
    println!("In our example");

    let t = TestStruct::new(1234);
    t.do_a_thing();
    assert_eq!(t.get_thing(), 1234);
}
