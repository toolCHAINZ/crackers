use cc::Build;
use derive_builder::Builder;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct AssemblyParameters {
    target: String,
    #[builder(setter(strip_option))]
    compiler: Option<String>,
    host: String,
}

impl AssemblyParameters {
    pub fn assemble(&self, dir: &TempDir, file: &NamedTempFile) -> Result<Vec<PathBuf>, cc::Error> {
        let mut build = Build::new();
        // We aren't running in a build script, so no need to print all those cargo directives
        // and environment variables
        build
            .cargo_warnings(false)
            .cargo_metadata(false)
            .cargo_debug(false)
            // location of the file to assemble
            .file(file.path())
            // directory to place the built object
            .out_dir(dir.path())
            // target architecture/os triple
            .target(&self.target)
            .opt_level(0)
            .host(&self.host);
        if let Some(c) = &self.compiler {
            // name of the compiler to use
            // necessary on macOS because
            build.compiler(c);
        }
        build.try_compile_intermediates()
    }
}

#[cfg(test)]
mod tests {
    use crate::specification::AssemblyParametersBuilder;

    #[test]
    fn simple_test() {
        let params = AssemblyParametersBuilder::default()
            .target("x86_64-unknown-linux-gnu")
            .compiler("x86_64-unknown-linux-gnu-gcc")
            .host("aarch64-apple-darwin")
            .build()
            .unwrap();
        //params.assemble("src/specification", "src/specification/test.S").unwrap();
    }
}
