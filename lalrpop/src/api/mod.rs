use crate::build;
use crate::log::Level;
use crate::session::{ColorConfig, Session};
use std::default::Default;
use std::env;
use std::env::current_dir;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[cfg(test)]
mod test;

/// Configure various aspects of how LALRPOP works.
/// Intended for use within a `build.rs` script.
/// To get the default configuration, use `Configuration::new`.
#[derive(Clone, Default)]
pub struct Configuration {
    session: Session,
}

impl Configuration {
    /// Creates the default configuration.
    ///
    /// equivalent to `Configuration::default`.
    pub fn new() -> Configuration {
        Configuration::default()
    }

    /// Always use ANSI colors in output, even if output does not appear to be a TTY.
    pub fn always_use_colors(&mut self) -> &mut Configuration {
        self.session.color_config = ColorConfig::Yes;
        self
    }

    /// Never use ANSI colors in output, even if output appears to be a TTY.
    pub fn never_use_colors(&mut self) -> &mut Configuration {
        self.session.color_config = ColorConfig::No;
        self
    }

    /// Use ANSI colors in output if output appears to be a TTY, but
    /// not otherwise. This is the default.
    pub fn use_colors_if_tty(&mut self) -> &mut Configuration {
        self.session.color_config = ColorConfig::IfTty;
        self
    }

    /// Specify a custom directory to search for input files.
    ///
    /// This directory is recursively searched for `.lalrpop` files to be
    /// considered as input files.  This configuration setting also
    /// impacts where output files are placed; paths are made relative
    /// to the input path before being resolved relative to the output
    /// path.  By default, the input directory is the current working
    /// directory.
    pub fn set_in_dir<P>(&mut self, dir: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.session.in_dir = Some(dir.into());
        self
    }

    /// Specify a custom directory to use when writing output files.
    ///
    /// By default, the output directory is the same as the input
    /// directory.
    pub fn set_out_dir<P>(&mut self, dir: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.session.out_dir = Some(dir.into());
        self
    }

    /// Apply `cargo` directory location conventions.
    ///
    /// This sets the input directory to `src` and the output directory to
    /// `$OUT_DIR`.
    pub fn use_cargo_dir_conventions(&mut self) -> &mut Self {
        self.set_in_dir("src")
            .set_out_dir(env::var("OUT_DIR").unwrap());
        self
    }

    /// Write output files in the same directory of the input files.
    ///
    /// If this option is enabled, you have to load the parser as a module:
    ///
    /// ```no_run
    /// mod parser; // synthesized from parser.lalrpop
    /// ```
    ///
    /// This was the default behaviour up to version 0.15.
    pub fn generate_in_source_tree(&mut self) -> &mut Self {
        self.set_in_dir(Path::new(".")).set_out_dir(Path::new("."))
    }

    /// If true, always convert `.lalrpop` files into `.rs` files, even if the
    /// `.rs` file is newer. Default is false.
    pub fn force_build(&mut self, val: bool) -> &mut Configuration {
        self.session.force_build = val;
        self
    }

    /// If true, print `rerun-if-changed` directives to standard output.
    ///
    /// If this is set, Cargo will only rerun the build script if any of the processed
    /// `.lalrpop` files are changed. This option is independent of
    /// [`Self::force_build()`], although it would be usual to set [`Self::force_build()`] and
    /// [`Self::emit_rerun_directives()`] at the same time.
    ///
    /// While many build scripts will want to set this to `true`, the default is
    /// false, because emitting any rerun directives to Cargo will cause the
    /// script to only be rerun when Cargo thinks it is needed. This could lead
    /// to hard-to-find bugs if other parts of the build script do not emit
    /// directives correctly, or need to be rerun unconditionally.
    pub fn emit_rerun_directives(&mut self, val: bool) -> &mut Configuration {
        self.session.emit_rerun_directives = val;
        self
    }

    /// If true, emit comments into the generated code.
    ///
    /// This makes the generated code significantly larger. Default is false.
    pub fn emit_comments(&mut self, val: bool) -> &mut Configuration {
        self.session.emit_comments = val;
        self
    }

    /// If false, shrinks the generated code by removing redundant white space.
    /// Default is true.
    pub fn emit_whitespace(&mut self, val: bool) -> &mut Configuration {
        self.session.emit_whitespace = val;
        self
    }

    /// If true, emit report file about generated code.
    pub fn emit_report(&mut self, val: bool) -> &mut Configuration {
        self.session.emit_report = val;
        self
    }

    /// Minimal logs: only for errors that halt progress.
    pub fn log_quiet(&mut self) -> &mut Configuration {
        self.session.log.set_level(Level::Taciturn);
        self
    }

    /// Informative logs: give some high-level indications of
    /// progress (default).
    pub fn log_info(&mut self) -> &mut Configuration {
        self.session.log.set_level(Level::Informative);
        self
    }

    /// Verbose logs: more than info, but still not overwhelming.
    pub fn log_verbose(&mut self) -> &mut Configuration {
        self.session.log.set_level(Level::Verbose);
        self
    }

    /// Debug logs: better redirect this to a file. Intended for
    /// debugging LALRPOP itself.
    pub fn log_debug(&mut self) -> &mut Configuration {
        self.session.log.set_level(Level::Debug);
        self
    }

    /// Set the max macro recursion depth.
    ///
    /// As lalrpop is resolving a macro, it may discover new macros uses in the
    /// macro definition to resolve.  Typically deep recursion indicates a
    /// recursive macro use that is non-resolvable.  The default resolution
    /// depth is 200.
    pub fn set_macro_recursion_limit(&mut self, val: u16) -> &mut Configuration {
        self.session.macro_recursion_limit = val;
        self
    }

    /// Sets the features used during compilation, disables the use of cargo features.
    /// (Default: Loaded from `CARGO_FEATURE_{}` environment variables).
    pub fn set_features<I>(&mut self, iterable: I) -> &mut Configuration
    where
        I: IntoIterator<Item = String>,
    {
        self.session.features = Some(iterable.into_iter().collect());
        self
    }

    /// Enables "unit-testing" configuration. This is only for
    /// lalrpop-test.
    #[doc(hidden)]
    pub fn unit_test(&mut self) -> &mut Configuration {
        self.session.unit_test = true;
        self
    }

    /// Process all files according to the `set_in_dir` and
    /// `set_out_dir` configuration.
    pub fn process(&self) -> Result<(), Box<dyn Error>> {
        let root = if let Some(ref d) = self.session.in_dir {
            d.as_path()
        } else {
            Path::new(".")
        };
        self.process_dir(root)
    }

    /// Process all files in the current directory, which -- unless you
    /// have changed it -- is typically the root of the crate being compiled.
    pub fn process_current_dir(&self) -> Result<(), Box<dyn Error>> {
        // If we can get a current dir, check to make sure session.in_dir either *wasn't* set, or
        // wasn't set to that dir.  If we can't get a current dir, we'll error out in a moment
        // anyways, and that's the bigger problem.
        if let Ok(current_dir) = current_dir() {
            if self.session.in_dir.is_some() && self.session.in_dir != Some(current_dir) {
                eprintln!("Error: \"process_current_dir()\" contradicts previously set in_dir");
                return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "\"process_current_dir()\" contradicts previously set in_dir.  Either use `process()` instead, or omit `set_in_dir()`.  (Note: in previous versions of lalrpop, this combination could affect the parser output dir.  If you were relying on this behavior to output the parser in your source directory, you may want to use `set_out_dir()` to retain that behavior.")));
            }
        }
        self.process_dir(current_dir()?)
    }

    /// Process all `.lalrpop` files in `path`.
    pub fn process_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut session = self.session.clone();

        // If out dir is empty, use cargo conventions by default.
        // See https://github.com/lalrpop/lalrpop/issues/280
        if session.out_dir.is_none() {
            let out_dir = env::var_os("OUT_DIR").ok_or("missing OUT_DIR variable")?;
            session.out_dir = Some(PathBuf::from(out_dir));
        }

        if self.session.features.is_none() {
            // Pick up the features cargo sets for build scripts
            session.features = Some(
                env::vars()
                    .filter_map(|(feature_var, _)| {
                        let prefix = "CARGO_FEATURE_";
                        feature_var
                            .strip_prefix(prefix)
                            .map(|feature| feature.replace('_', "-").to_ascii_lowercase())
                    })
                    .collect(),
            );
        }

        let session = Rc::new(session);
        build::process_dir(session, path)?;
        Ok(())
    }

    /// Process the given `.lalrpop` file.
    pub fn process_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let session = Rc::new(self.session.clone());
        build::process_file(session, path)?;
        Ok(())
    }
}

/// Process all files in the current directory.
///
/// Unless you have changed it this is typically the root of the crate being compiled.
/// If your project only builds one crate and your files are in a ./src directory, you should use
/// `process_src()` instead
///
/// Equivalent to `Configuration::new().process_current_dir()`.
pub fn process_root() -> Result<(), Box<dyn Error>> {
    Configuration::new().process_current_dir()
}

/// Process all files in ./src.
///
/// In many cargo projects which build only one crate, this is the normal
/// location for source files.  If you are running lalrpop from a top level build.rs in a
/// project that builds multiple crates, you may want `process_root()` instead.
/// See `Configuration` if you would like more fine-grain control over lalrpop.
pub fn process_src() -> Result<(), Box<dyn Error>> {
    Configuration::new().set_in_dir("./src").process()
}

/// Deprecated in favor of `Configuration`.
///
/// Instead, consider using:
///
/// ```rust
/// Configuration::new().force_build(true).process_current_dir()
/// ```
///
#[deprecated(
    since = "1.0.0",
    note = "use `Configuration::new().force_build(true).process_current_dir()` instead"
)]
pub fn process_root_unconditionally() -> Result<(), Box<dyn Error>> {
    Configuration::new().force_build(true).process_current_dir()
}
