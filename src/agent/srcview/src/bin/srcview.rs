// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use anyhow::{format_err, Context, Result};
use clap::Parser;
use srcview::{ModOff, Report, SrcLine, SrcView};
use std::fs::{self, OpenOptions};
use std::io::{stdout, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
enum Opt {
    Srcloc(SrcLocOpt),
    PdbPaths(PdbPathsOpt),
    Cobertura(CoberturaOpt),
    /// Print 3rd-party license information
    Licenses,
}

/// Print the file paths in the provided PDB
#[derive(Parser, Debug)]
struct PdbPathsOpt {
    pdb_path: PathBuf,
}

/// Print modoffset file with file and source lines
#[derive(Parser, Debug)]
struct SrcLocOpt {
    pdb_path: PathBuf,
    modoff_path: PathBuf,
    #[arg(long)]
    module_name: Option<String>,
}

/// Generate a Cobertura XML coverage report
///
/// Example:
///   srcview cobertura ./res/example.pdb res/example.txt -
///             --include-regex "E:\\\\1f\\\\coverage\\\\"
///             --filter-regex "E:\\\\1f\\\\coverage\\\\"
///             --module-name example.exe
///
/// In this example, only files that live in E:\1f\coverage are included and
/// E:\1f\coverage is removed from the filenames in the resulting XML report.
///
/// The XML report is written to either a file or stdout if the argument is
/// a single dash.
#[derive(Parser, Debug)]
struct CoberturaOpt {
    pdb_path: PathBuf,
    modoff_path: PathBuf,
    #[arg(default_value = "-")]
    output_path: String,
    #[arg(long)]
    module_name: Option<String>,

    /// regular expression that will be applied against the file paths from the
    /// srcview
    #[arg(long)]
    include_regex: Option<String>,

    /// search and replace regular expression that is applied to all file
    /// paths that will appear in the output report
    #[arg(long)]
    filter_regex: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::parse();

    match opt {
        Opt::Srcloc(opts) => srcloc(opts)?,
        Opt::PdbPaths(opts) => pdb_paths(opts)?,
        Opt::Cobertura(opts) => cobertura(opts)?,
        Opt::Licenses => licenses()?,
    };

    Ok(())
}

fn licenses() -> Result<()> {
    stdout().write_all(include_bytes!("../../../data/licenses.json"))?;
    Ok(())
}

// In the case the user did not specify the module name of interest, this
// utility function will guess at the module name based on the PDB path name.
//
// This is a last-ditch effort to ensure the coverage report has something
// consumable.
fn add_common_extensions(srcview: &mut SrcView, pdb_path: &Path) -> Result<()> {
    let pdb_file_name = pdb_path.file_name().ok_or_else(|| {
        format_err!(
            "unable to identify file name from path: {}",
            pdb_path.display()
        )
    })?;

    let stem = Path::new(pdb_file_name)
        .file_stem()
        .ok_or_else(|| {
            format_err!(
                "unable to identify file stem from path: {}",
                pdb_path.display()
            )
        })?
        .to_string_lossy();

    // add module without extension
    srcview.insert(&stem, pdb_path)?;
    // add common module extensions
    for ext in ["sys", "exe", "dll"] {
        srcview.insert(&format!("{stem}.{ext}"), pdb_path)?;
    }
    Ok(())
}

fn srcloc(opts: SrcLocOpt) -> Result<()> {
    let modoff_data = fs::read_to_string(&opts.modoff_path)
        .with_context(|| format!("unable to read modoff_path: {}", opts.modoff_path.display()))?;
    let modoffs = ModOff::parse(&modoff_data)?;
    let mut srcview = SrcView::new();

    if let Some(module_name) = &opts.module_name {
        srcview.insert(module_name, &opts.pdb_path)?;
    } else {
        add_common_extensions(&mut srcview, &opts.pdb_path)?;
    }

    for modoff in &modoffs {
        print!(" +{:04x} ", modoff.offset);
        match srcview.modoff(modoff) {
            Some(srcloc) => println!("{srcloc}"),
            None => println!(),
        }
    }

    Ok(())
}

fn pdb_paths(opts: PdbPathsOpt) -> Result<()> {
    let mut srcview = SrcView::new();
    srcview.insert(&opts.pdb_path.to_string_lossy(), &opts.pdb_path)?;

    for path in srcview.paths() {
        println!("{}", path.display());
    }
    Ok(())
}

fn cobertura(opts: CoberturaOpt) -> Result<()> {
    // read our modoff file and parse it to a vector
    let modoff_data = fs::read_to_string(&opts.modoff_path)?;
    let modoffs = ModOff::parse(&modoff_data)?;

    let mut output_writer = match opts.output_path.as_str() {
        "-" => Box::new(BufWriter::new(stdout())) as Box<dyn Write>,
        path => {
            let path = Path::new(path);

            Box::new(BufWriter::with_capacity(
                0x10_0000, // 1MB
                OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(path)?,
            )) as Box<dyn Write>
        }
    };

    // create our new SrcView and insert our only pdb into it
    // we don't know what the modoff module will be, so create a mapping from
    // all likely names to the pdb
    let mut srcview = SrcView::new();

    if let Some(module_name) = &opts.module_name {
        srcview.insert(module_name, &opts.pdb_path)?;
    } else {
        add_common_extensions(&mut srcview, &opts.pdb_path)?;
    }

    // Convert our ModOffs to SrcLine so we can draw it
    let coverage: Vec<SrcLine> = modoffs
        .into_iter()
        .filter_map(|m| srcview.modoff(&m))
        .collect();

    // Generate our report, filtering on our example path
    let r = Report::new(&coverage, &srcview, opts.include_regex.as_deref())?;

    // Format it as cobertura and display it
    r.cobertura(opts.filter_regex.as_deref(), &mut output_writer)?;
    Ok(())
}
