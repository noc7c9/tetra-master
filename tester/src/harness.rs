use owo_colors::OwoColorize;
use std::panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe};

const INDENT: usize = 2;

#[macro_export]
macro_rules! test {
    (@$harness:ident.$method:ident; $name:literal; |_| $test:expr) => {{
        $harness.$method($name, |_| { $test; Ok(()) })
    }};
    (@$harness:ident.$method:ident; $name:literal; |$ctx:ident| $test:expr) => {{
        $harness.$method($name, |$ctx| { $test; Ok(()) })
    }};
    ($harness:ident $name:literal; |_| $test:expr) => {
        test!(@$harness.test; $name; |_| $test)
    };
    ($harness:ident $name:literal; |$ctx:ident| $test:expr) => {
        test!(@$harness.test; $name; |$ctx| $test)
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! focus_test {
    ($harness:ident $name:literal; |_| $test:expr) => {
        test!(@$harness.test_focus; $name; |_| $test)
    };
    ($harness:ident $name:literal; |$ctx:ident| $test:expr) => {
        test!(@$harness.test_focus; $name; |$ctx| $test)
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! skip_test {
    ($harness:ident $name:literal; |_| $test:expr) => {
        test!(@$harness.test_skip; $name; |_| $test)
    };
    ($harness:ident $name:literal; |$ctx:ident| $test:expr) => {
        test!(@$harness.test_skip; $name; |$ctx| $test)
    };
}

#[macro_export]
macro_rules! suite {
    (@$harness:ident.$method:ident; $name:literal) => {{ $harness.$method($name) }};
    ($harness:ident $name:literal) => { suite!(@$harness.suite; $name) };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! focus_suite {
    ($harness:ident $name:literal) => { suite!(@$harness.suite_focus; $name) };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! skip_suite {
    ($harness:ident $name:literal) => { suite!(@$harness.suite_skip; $name) };
}

type Name = &'static str;
type TestFn<Ctx> = Box<dyn FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe>;

#[derive(Debug, Clone, Copy)]
enum Marker {
    None,
    Focused,
    Skipped,
}

#[derive(Debug, Default)]
struct Counts {
    total: usize,
    skipped: usize,
    passed: usize,
    failed: usize,
}

struct State<Ctx> {
    ctx: Ctx,
    indent: String,
    has_focused_items: bool,
    ancestor_markers: Vec<Marker>,
}

impl<Ctx> State<Ctx> {
    fn indent(&mut self, marker: Marker) {
        self.ancestor_markers.push(marker);
        for _ in 0..INDENT {
            self.indent.push(' ');
        }
    }

    fn dedent(&mut self) {
        self.ancestor_markers.pop();
        for _ in 0..INDENT {
            self.indent.pop();
        }
    }

    fn has_focused_ancestor(&self) -> bool {
        self.ancestor_markers
            .iter()
            .any(|m| matches!(m, Marker::Focused))
    }

    fn has_skipped_ancestor(&self) -> bool {
        self.ancestor_markers
            .iter()
            .any(|m| matches!(m, Marker::Skipped))
    }
}

pub(crate) struct Harness<Ctx> {
    state: State<Ctx>,
    inner: Suite<Ctx>,
}

impl<Ctx: RefUnwindSafe> Harness<Ctx> {
    pub(crate) fn new(ctx: Ctx) -> Self {
        Self {
            state: State {
                ctx,
                indent: String::new(),
                has_focused_items: false,
                ancestor_markers: Vec::new(),
            },
            inner: Suite {
                name: None,
                marker: Marker::None,
                items: Vec::new(),
            },
        }
    }

    #[allow(dead_code)]
    pub(crate) fn test(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.inner.push_test(name, Marker::None, Box::new(func));
    }

    #[allow(dead_code)]
    pub(crate) fn test_focus(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.inner.push_test(name, Marker::Focused, Box::new(func));
    }

    #[allow(dead_code)]
    pub(crate) fn test_skip(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.inner.push_test(name, Marker::Skipped, Box::new(func));
    }

    #[allow(dead_code)]
    pub(crate) fn suite(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.inner.suite(name)
    }

    #[allow(dead_code)]
    pub(crate) fn suite_focus(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.inner.suite_focus(name)
    }

    #[allow(dead_code)]
    pub(crate) fn suite_skip(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.inner.suite_skip(name)
    }

    pub(crate) fn run(mut self) -> Ctx {
        println!("Running tests...\n");

        self.state.has_focused_items = self.inner.has_focused_items();

        let counts = self.inner.run(&mut self.state);

        println!();
        if counts.failed > 0 {
            print!("{} {}, ", counts.failed.red(), "failed".red());
        }
        if counts.skipped > 0 {
            print!("{} {}, ", counts.skipped.yellow(), "skipped".yellow());
        }
        if counts.passed > 0 {
            print!("{} {}, ", counts.passed.green(), "passed".green());
        }
        println!("{} total", counts.total);

        self.state.ctx
    }
}

struct Test<Ctx> {
    name: Name,
    marker: Marker,
    func: TestFn<Ctx>,
}

enum TestResult {
    Skipped,
    Passed,
    Failed,
}

impl<Ctx: RefUnwindSafe> Test<Ctx> {
    fn is_focused(&self) -> bool {
        matches!(self.marker, Marker::Focused)
    }

    fn is_skipped(&self) -> bool {
        matches!(self.marker, Marker::Skipped)
    }

    fn run(self, state: &mut State<Ctx>) -> TestResult {
        let is_skipped = if self.is_skipped() {
            true
        } else if (self.is_focused() || state.has_focused_ancestor())
            && !state.has_skipped_ancestor()
        {
            false
        } else if state.has_skipped_ancestor() {
            true
        } else {
            state.has_focused_items
        };

        if is_skipped {
            println!("{}{} {}", state.indent, "SKIP".yellow(), self.name);
            return TestResult::Skipped;
        }

        let mut ctx = AssertUnwindSafe(&mut state.ctx);
        match std::panic::catch_unwind(move || (self.func)(&mut ctx).unwrap()) {
            Err(err) => {
                println!("{}{} {}: {err:?}", state.indent, "FAIL".red(), self.name);
                TestResult::Failed
            }
            Ok(_) => {
                println!("{}{} {}", state.indent, "PASS".green(), self.name);
                TestResult::Passed
            }
        }
    }
}

enum Item<Ctx> {
    Test(Test<Ctx>),
    Suite(Suite<Ctx>),
}

pub(crate) struct Suite<Ctx> {
    name: Option<Name>,
    marker: Marker,
    items: Vec<Item<Ctx>>,
}

impl<Ctx: RefUnwindSafe> Suite<Ctx> {
    #[allow(dead_code)]
    fn push_test(&mut self, name: Name, marker: Marker, func: TestFn<Ctx>) {
        self.items.push(Item::Test(Test { name, marker, func }));
    }

    #[allow(dead_code)]
    pub(crate) fn test(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.push_test(name, Marker::None, Box::new(func));
    }

    #[allow(dead_code)]
    pub(crate) fn test_focus(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.push_test(name, Marker::Focused, Box::new(func));
    }

    #[allow(dead_code)]
    pub(crate) fn test_skip(
        &mut self,
        name: Name,
        func: impl FnOnce(&mut Ctx) -> anyhow::Result<()> + UnwindSafe + 'static,
    ) {
        self.push_test(name, Marker::Skipped, Box::new(func));
    }

    #[allow(dead_code)]
    fn push_suite(&mut self, name: Name, marker: Marker) -> &mut Suite<Ctx> {
        self.items.push(Item::Suite(Suite {
            name: Some(name),
            marker,
            items: Vec::new(),
        }));
        if let Some(Item::Suite(suite)) = self.items.last_mut() {
            suite
        } else {
            unreachable!()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn suite(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.push_suite(name, Marker::None)
    }

    #[allow(dead_code)]
    pub(crate) fn suite_focus(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.push_suite(name, Marker::Focused)
    }

    #[allow(dead_code)]
    pub(crate) fn suite_skip(&mut self, name: Name) -> &mut Suite<Ctx> {
        self.push_suite(name, Marker::Skipped)
    }

    fn has_focused_items(&self) -> bool {
        self.items.iter().any(|item| match item {
            Item::Suite(suite) => {
                suite.is_focused() || (!suite.is_skipped() && suite.has_focused_items())
            }
            Item::Test(test) => test.is_focused(),
        })
    }

    fn is_focused(&self) -> bool {
        matches!(self.marker, Marker::Focused)
    }

    fn is_skipped(&self) -> bool {
        matches!(self.marker, Marker::Skipped)
    }

    fn run(self, state: &mut State<Ctx>) -> Counts {
        let mut counts = Counts::default();

        if let Some(name) = self.name {
            print!("{}", &state.indent);
            if self.is_skipped() || state.has_skipped_ancestor() {
                print!("{} ", "SKIP".yellow(),);
            }
            println!("{} {}", "SUITE".purple(), name);
        }

        // state for leaving blank lines around suite runs
        let mut prev_run_was_suite = false;
        let mut suite_run_just_started = true;

        state.indent(self.marker);
        for item in self.items {
            match item {
                Item::Suite(suite) => {
                    // don't have a blank line if this is the first item under a suite
                    if !suite_run_just_started {
                        println!();
                    }

                    let c = suite.run(state);
                    counts.total += c.total;
                    counts.skipped += c.skipped;
                    counts.passed += c.passed;
                    counts.failed += c.failed;

                    prev_run_was_suite = true;
                }
                Item::Test(test) => {
                    if prev_run_was_suite {
                        println!();
                    }
                    prev_run_was_suite = false;

                    match test.run(state) {
                        TestResult::Passed => {
                            counts.passed += 1;
                        }
                        TestResult::Failed => {
                            counts.failed += 1;
                        }
                        TestResult::Skipped => {
                            counts.skipped += 1;
                        }
                    }
                    counts.total += 1;
                }
            }

            suite_run_just_started = false;
        }
        state.dedent();

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_at_root() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));
        test!(h "b"; |m| m.push("b"));
        test!(h "c"; |m| m.push("c"));

        assert_eq!(h.run(), vec!["a", "b", "c"]);
    }

    #[test]
    fn skipped_tests_at_root() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));
        skip_test!(h "b"; |m| m.push("b"));
        test!(h "c"; |m| m.push("c"));
        skip_test!(h "d"; |m| m.push("d"));

        assert_eq!(h.run(), vec!["a", "c"]);
    }

    #[test]
    fn one_focused_test_at_root() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));
        focus_test!(h "b"; |m| m.push("b"));
        test!(h "c"; |m| m.push("c"));

        assert_eq!(h.run(), vec!["b"]);
    }

    #[test]
    fn many_focused_tests_at_root() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));
        focus_test!(h "b"; |m| m.push("b"));
        test!(h "c"; |m| m.push("c"));
        test!(h "d"; |m| m.push("d"));
        focus_test!(h "e"; |m| m.push("e"));

        assert_eq!(h.run(), vec!["b", "e"]);
    }

    #[test]
    fn focused_and_skipped_tests_at_root() {
        let mut h = Harness::new(vec![]);

        skip_test!(h "a"; |m| m.push("a"));
        focus_test!(h "b"; |m| m.push("b"));
        test!(h "c"; |m| m.push("c"));
        test!(h "d"; |m| m.push("d"));
        skip_test!(h "e"; |m| m.push("e"));

        assert_eq!(h.run(), vec!["b"]);
    }

    #[test]
    fn tests_in_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "S a", "S b", "S c", "b"]);
    }

    #[test]
    fn skipped_tests_in_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            skip_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
            skip_test!(s "S d"; |m| m.push("S d"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "S a", "S c", "b"]);
    }

    #[test]
    fn one_focused_test_in_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            focus_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S b"]);
    }

    #[test]
    fn many_focused_tests_in_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            focus_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
            test!(s "S d"; |m| m.push("S d"));
            focus_test!(s "S e"; |m| m.push("S e"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S b", "S e"]);
    }

    #[test]
    fn focused_and_skipped_tests_in_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            skip_test!(s "S a"; |m| m.push("S a"));
            focus_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
            test!(s "S d"; |m| m.push("S d"));
            skip_test!(s "S e"; |m| m.push("S e"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S b"]);
    }

    #[test]
    fn multiple_focused_tests_in_nested_suites() {
        let mut h = Harness::new(vec![]);

        focus_test!(h "a"; |m| m.push("a"));

        {
            let s = suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            focus_test!(s "S b"; |m| m.push("S b"));

            {
                let ss = suite!(s "SS");
                focus_test!(ss "SS a"; |m| m.push("SS a"));
                test!(ss "SS b"; |m| m.push("SS b"));
                focus_test!(ss "SS c"; |m| m.push("SS c"));
            }

            focus_test!(s "S c"; |m| m.push("S c"));
        }

        assert_eq!(h.run(), vec!["a", "S b", "SS a", "SS c", "S c"]);
    }

    #[test]
    fn deeply_nested_focused_test() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));
        {
            let s = suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            {
                let ss = suite!(s "SS");
                test!(ss "SS a"; |m| m.push("SS a"));
                {
                    let sss = suite!(ss "SSS");
                    test!(sss "SSS a"; |m| m.push("SSS a"));
                    {
                        let ssss = suite!(sss "SSSS");
                        test!(ssss "SSSS a"; |m| m.push("SSSS a"));
                        {
                            let sssss = suite!(ssss "SSSSS");
                            focus_test!(sssss "SSSSS focused"; |m| m.push("SSSSS focused"));
                        }
                        test!(ssss "SSSS b"; |m| m.push("SSSS b"));
                    }
                    test!(sss "SSS b"; |m| m.push("SSS b"));
                }
                test!(ss "SS a"; |m| m.push("SS a"));
            }
            test!(s "S b"; |m| m.push("S b"));
        }
        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["SSSSS focused"]);
    }

    #[test]
    fn skipped_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = skip_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "b"]);
    }

    #[test]
    fn focused_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = focus_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S a", "S b", "S c"]);
    }

    #[test]
    fn skipped_test_in_focused_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = focus_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            skip_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S a", "S c"]);
    }

    #[test]
    fn focused_test_in_skipped_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = skip_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            focus_test!(s "S b"; |m| m.push("S b"));
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "b"]);
    }

    #[test]
    fn nested_focused_test_in_skipped_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        let s = skip_suite!(h "S");
        test!(s "S a"; |m| m.push("S a"));
        {
            let ss = suite!(s "SS");
            test!(ss "SS a"; |m| m.push("SS a"));
            {
                let sss = suite!(ss "SSS");
                focus_test!(sss "SSS b"; |m| m.push("SSS b"));
            }
            test!(ss "SS b"; |m| m.push("SS b"));
        }
        test!(s "S b"; |m| m.push("S b"));

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "b"]);
    }

    #[test]
    fn skipped_suite_in_focused_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = focus_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            {
                let ss = skip_suite!(s "SS");
                test!(ss "SS a"; |m| m.push("SS a"));
                test!(ss "SS b"; |m| m.push("SS b"));
                test!(ss "SS c"; |m| m.push("SS c"));
            }
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["S a", "S c"]);
    }

    #[test]
    fn focused_suite_in_skipped_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        {
            let s = skip_suite!(h "S");
            test!(s "S a"; |m| m.push("S a"));
            {
                let ss = focus_suite!(s "SS");
                test!(ss "SS a"; |m| m.push("SS a"));
                test!(ss "SS b"; |m| m.push("SS b"));
                test!(ss "SS c"; |m| m.push("SS c"));
            }
            test!(s "S c"; |m| m.push("S c"));
        }

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "b"]);
    }

    #[test]
    fn nested_focused_suite_in_skipped_suite() {
        let mut h = Harness::new(vec![]);

        test!(h "a"; |m| m.push("a"));

        let s = skip_suite!(h "S");
        test!(s "S a"; |m| m.push("S a"));
        {
            let ss = suite!(s "SS");
            test!(ss "SS a"; |m| m.push("SS a"));
            {
                let sss = suite!(ss "SSS");
                focus_test!(sss "SSS b"; |m| m.push("SSS b"));
                {
                    let ssss = focus_suite!(sss "SSSS");
                    test!(ssss "SSSS a"; |m| m.push("SSSS a"));
                    test!(ssss "SSSS b"; |m| m.push("SSSS b"));
                    test!(ssss "SSSS c"; |m| m.push("SSSS c"));
                }
            }
            test!(ss "SS b"; |m| m.push("SS b"));
        }
        test!(s "S b"; |m| m.push("S b"));

        test!(h "b"; |m| m.push("b"));

        assert_eq!(h.run(), vec!["a", "b"]);
    }
}
