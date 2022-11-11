pub use inner::*;

#[cfg(not(target_arch = "wasm32"))]
mod inner {
    pub struct Instant(std::time::Instant);

    impl Instant {
        pub fn now() -> Self {
            Self(std::time::Instant::now())
        }

        pub fn elapsed(&self) -> u64 {
            self.0.elapsed().as_millis() as u64
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod inner {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = "export function performance_now() { return performance.now(); }")]
    extern "C" {
        fn performance_now() -> f64;
    }

    pub struct Instant(f64);

    impl Instant {
        pub fn now() -> Self {
            Self(performance_now())
        }

        pub fn elapsed(&self) -> u64 {
            (performance_now() - self.0) as u64
        }
    }
}
