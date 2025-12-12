use std::{env, error::Error};

use vergen::Emitter;
use vergen_git2::Git2Builder;

fn main() -> Result<(), Box<dyn Error>> {
    let git2 = Git2Builder::default()
        .describe(
            // tags=
            true,  // dirty=
            false, // matches=
            None,
        )
        .sha(false)
        .build()?;
    Emitter::default().add_instructions(&git2)?.emit_and_set()?;

    let git_describe = env::var("VERGEN_GIT_DESCRIBE")?;
    let mangatan_version = git_describe.split('-').collect::<Vec<&str>>()[0];
    println!("cargo:rustc-env=MANGATAN_VERSION={mangatan_version}");

    Ok(())
}
