#![allow(dead_code)]

pub use inner::*;

#[cfg(feature = "metrics")]
mod inner {
    use std::collections::HashMap;

    #[derive(Debug, Default)]
    pub struct Metrics {
        name: &'static str,
        number_of_expended_nodes: usize,
        number_of_depth_limit_leafs_reached: usize,
        number_of_terminal_leafs_reached: usize,
        number_of_pruned_nodes: HashMap<usize, usize>, // depth, count
    }

    impl Metrics {
        pub fn new(name: &'static str) -> Self {
            Self {
                name,
                ..Default::default()
            }
        }

        pub fn inc_expanded_nodes(&mut self) {
            self.number_of_expended_nodes += 1;
        }

        pub fn inc_depth_limit_leafs(&mut self) {
            self.number_of_depth_limit_leafs_reached += 1;
        }

        pub fn inc_terminal_leafs(&mut self) {
            self.number_of_terminal_leafs_reached += 1;
        }

        pub fn inc_pruned_nodes(&mut self, depth: usize) {
            *self.number_of_pruned_nodes.entry(depth).or_default() += 1;
        }

        pub fn print_report(&self) {
            println!("{:#?}", self);
        }
    }
}

#[cfg(not(feature = "metrics"))]
mod inner {
    #[derive(Debug, Default)]
    pub struct Metrics;

    impl Metrics {
        pub fn new(_name: &'static str) -> Self {
            Self
        }

        pub fn inc_expanded_nodes(&mut self) {}

        pub fn inc_depth_limit_leafs(&mut self) {}

        pub fn inc_terminal_leafs(&mut self) {}

        pub fn inc_pruned_nodes(&mut self, _depth: usize) {}

        pub fn print_report(&self) {}
    }
}
