#[macro_use]
mod harness;

mod impl_tests;

#[derive(Debug, clap::Parser)]
struct Args {
    implementation: String,
}

fn main() -> anyhow::Result<()> {
    let args = {
        use clap::Parser;
        Args::parse()
    };

    impl_tests::run(args.implementation);

    Ok(())
}
