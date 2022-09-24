#[macro_use]
mod harness;

mod impl_tests;

#[derive(Debug, clap::Parser)]
struct Args {
    #[clap(value_name = "PATH")]
    /// Path to the external implementation to use, if omitted the reference implementation will be
    /// used
    implementation: Option<String>,
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let args = Args::parse();

    let ctx = if let Some(implementation) = &args.implementation {
        impl_tests::Ctx::External { implementation }
    } else {
        impl_tests::Ctx::Reference
    };

    impl_tests::run(ctx);

    Ok(())
}
