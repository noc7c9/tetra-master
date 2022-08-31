use std::panic::UnwindSafe;

struct Test<'a> {
    name: &'a str,
    marker: Marker,
    func: Box<dyn FnOnce() -> anyhow::Result<()> + UnwindSafe + 'a>,
}

enum Marker {
    None,
    Focused,
    Skipped,
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
            marker: Marker::None,
            func: Box::new(func),
        });
    }

    #[allow(dead_code)]
    pub(crate) fn test_focus(
        &mut self,
        name: &'a str,
        func: impl FnOnce() -> anyhow::Result<()> + UnwindSafe + 'a,
    ) {
        self.tests.push(Test {
            name,
            marker: Marker::Focused,
            func: Box::new(func),
        });
    }

    #[allow(dead_code)]
    pub(crate) fn test_skip(
        &mut self,
        name: &'a str,
        func: impl FnOnce() -> anyhow::Result<()> + UnwindSafe + 'a,
    ) {
        self.tests.push(Test {
            name,
            marker: Marker::Skipped,
            func: Box::new(func),
        });
    }

    pub(crate) fn run(self) {
        use owo_colors::OwoColorize;

        let has_focused_tests = self
            .tests
            .iter()
            .any(|test| matches!(test.marker, Marker::Focused));

        let mut total = 0;
        let mut skipped = 0;
        let mut passed = 0;
        let mut failed = 0;

        println!("Running tests...\n");

        for test in self.tests {
            total += 1;

            if matches!(test.marker, Marker::Skipped)
                || (has_focused_tests && matches!(test.marker, Marker::None))
            {
                skipped += 1;
                println!("   {} {}\n", "SKIP".yellow(), test.name);
                continue;
            }

            match std::panic::catch_unwind(move || (test.func)().unwrap()) {
                Err(err) => {
                    failed += 1;
                    println!("   {} {}: {err:?}\n", "FAIL".red(), test.name);
                }
                Ok(_) => {
                    passed += 1;
                    println!("   {} {}\n", "PASS".green(), test.name);
                }
            }
        }

        if failed > 0 {
            print!("{} {}, ", failed.red(), "failed".red());
        }
        if skipped > 0 {
            print!("{} {}, ", skipped.yellow(), "skipped".yellow());
        }
        if passed > 0 {
            print!("{} {}, ", passed.green(), "passed".green());
        }
        println!("{total} total");
    }
}
