//! Human-readable report formatting.

use fdupe::manifest::ExecutionReport;
use fdupe::model::ScanReport;

#[must_use]
#[allow(clippy::cast_precision_loss)] // human-readable sizes only; exactness beyond f64 mantissa is irrelevant
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

pub fn print_report(report: &ScanReport) {
    let s = &report.stats;
    println!("fdupe scan report");
    println!("==================");
    println!("files scanned:            {}", s.files_scanned);
    println!(
        "identical files (links):  {} ({} extra paths, not counted as duplicates)",
        s.identity_groups, s.identity_files_skipped
    );
    println!("eliminated by size stage: {} files (unique size)", s.unique_size_files);
    println!("sample-hashed:             {} files", s.sample_hashed);
    println!("eliminated by sample stage: {} files (unique sample)", s.unique_sample_files);
    println!("fully hashed:              {} files", s.fully_hashed);
    println!();
    println!("duplicate groups found:   {}", s.duplicate_groups);
    println!("duplicate files:          {}", s.duplicate_files);
    println!("reclaimable space:        {}", human_bytes(s.reclaimable_bytes));
    println!();

    if !report.identity_groups.is_empty() {
        println!("hard-linked / identical-inode groups:");
        for g in &report.identity_groups {
            println!("  [{}]", human_bytes(g.size));
            for p in &g.paths {
                println!("    = {}", p.display());
            }
        }
        println!();
    }

    if report.duplicate_groups.is_empty() {
        println!("no content duplicates found.");
        return;
    }

    println!("duplicate groups:");
    for g in &report.duplicate_groups {
        println!(
            "  [{}] {} copies, reclaim {} (hash {}...)",
            human_bytes(g.size),
            g.paths.len(),
            human_bytes(g.reclaimable_bytes()),
            &g.hash[..12.min(g.hash.len())]
        );
        for p in &g.paths {
            println!("    - {}", p.display());
        }
    }
}

pub fn print_execution_report(report: &ExecutionReport, dry_run: bool) {
    if dry_run {
        println!("dry run: would delete {} files", report.would_delete.len());
        for p in &report.would_delete {
            println!("  - {}", p.display());
        }
    } else {
        println!("deleted {} files", report.deleted.len());
        for p in &report.deleted {
            println!("  - {}", p.display());
        }
    }
    println!("space reclaimed: {}", human_bytes(report.bytes_reclaimed));

    if !report.skipped.is_empty() {
        println!("skipped:");
        for (p, reason) in &report.skipped {
            println!("  ! {} ({reason})", p.display());
        }
    }

    if !report.refused_groups.is_empty() {
        println!("refused:");
        for r in &report.refused_groups {
            println!("  ! {r}");
        }
    }
}
