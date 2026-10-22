//! `fdupe` command-line entry point.
#![forbid(unsafe_code)]

mod cli;
mod output;

use clap::Parser;
use cli::Cli;
use fdupe::delete::delete_duplicates;
use fdupe::manifest::{build_manifest, execute_manifest, read_manifest, write_manifest};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(cli: &Cli) -> anyhow::Result<()> {
    if let Some(manifest_path) = &cli.from_manifest {
        // Executing a manifest removes files, so it needs the same explicit
        // `--delete` as a direct delete (or `--dry-run` to preview).
        if !cli.delete && !cli.dry_run {
            anyhow::bail!(
                "--from-manifest needs --delete to remove files, or --dry-run to preview; nothing was changed"
            );
        }
        let manifest = read_manifest(manifest_path)?;
        let report = execute_manifest(&manifest, cli.dry_run, cli.skip_verify);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            output::print_execution_report(&report, cli.dry_run);
        }
        if !report.refused_groups.is_empty() {
            anyhow::bail!("{} group(s) refused; nothing in them was deleted", report.refused_groups.len());
        }
        return Ok(());
    }

    let report = fdupe::scan(&cli.paths, cli.min_size, cli.follow_symlinks, &cli.exclude, cli.sample_size)?;

    if let Some(manifest_path) = &cli.manifest {
        let strategy = cli.keep.ok_or_else(|| {
            anyhow::anyhow!("--manifest requires --keep <first|oldest|newest|shortest-path>")
        })?;
        let manifest = build_manifest(&report.duplicate_groups, strategy);
        write_manifest(&manifest, manifest_path)?;
        println!(
            "wrote review manifest with {} group(s) to {}",
            manifest.groups.len(),
            manifest_path.display()
        );
        if !cli.json {
            output::print_report(&report);
        }
        return Ok(());
    }

    if cli.delete {
        let strategy = cli
            .keep
            .ok_or_else(|| anyhow::anyhow!("--delete requires --keep <first|oldest|newest|shortest-path>"))?;
        let exec_report = delete_duplicates(&report.duplicate_groups, strategy, cli.dry_run, cli.skip_verify);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&exec_report)?);
        } else {
            output::print_execution_report(&exec_report, cli.dry_run);
        }
        if !exec_report.refused_groups.is_empty() {
            anyhow::bail!(
                "{} group(s) refused; nothing in them was deleted",
                exec_report.refused_groups.len()
            );
        }
        return Ok(());
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        output::print_report(&report);
    }

    Ok(())
}
