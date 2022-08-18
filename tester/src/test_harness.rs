use std::panic::UnwindSafe;

struct Test<'a> {
    name: &'a str,
    func: Box<dyn FnOnce() -> anyhow::Result<()> + UnwindSafe + 'a>,
}

pub(crate) struct Harness<'a> {
    tests: Vec<Test<'a>>,
}

impl<'a> Harness<'a> {
    pub(crate) fn new() -> Self {
        Self { tests: Vec::new() }
    }

    pub(crate) fn test(
        &mut self,
        name: &'a str,
        func: impl FnOnce() -> anyhow::Result<()> + UnwindSafe + 'a,
    ) {
        self.tests.push(Test {
            name,
            func: Box::new(func),
        });
    }

    pub(crate) fn run(self) {
        for test in self.tests {
            match std::panic::catch_unwind(move || (test.func)().unwrap()) {
                Err(err) => println!("> {} FAIL: {err:?}", test.name),
                Ok(_) => println!("> {} PASS", test.name),
            }
        }
    }
}
