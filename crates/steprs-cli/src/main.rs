use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use steprs::{analyze_step, parse_only, run_system_tests};

#[derive(Parser)]
#[command(
    name = "steprs",
    version,
    about = "STEP B-rep coaxial hole extractor — steprs.dev"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Header and record statistics
    Inspect {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// STEP → B-rep → coaxial hole detection
    Analyze {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Run cylinder_block + AP214 fixture regression checks
    Test,
    /// List supported manufacturing feature kinds
    Catalog,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Catalog => {
            println!("Manufacturing feature catalog:\n");
            for kind in steprs_core::ManufacturingFeatureKind::MVP {
                println!("  • {} ({:?})", kind.label(), kind);
            }
        }
        Commands::Test => {
            let status = run_system_tests();
            for case in &status.cases {
                let mark = if case.pass { "PASS" } else { "FAIL" };
                println!("{mark}  {} — {}", case.name, case.detail);
            }
            if !status.all_pass {
                std::process::exit(1);
            }
        }
        Commands::Inspect { path, json } => {
            let content = fs::read_to_string(&path)?;
            let stats = parse_only(&content)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                print_stats(&stats);
            }
        }
        Commands::Analyze { path, json } => {
            let content = fs::read_to_string(&path)?;
            let result = analyze_step(&content)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                print_analysis(&result);
            }
        }
    }

    Ok(())
}

fn print_stats(stats: &steprs_core::ParseStats) {
    println!("Schema:       {}", stats.schema);
    println!("Protocol:     {}", stats.application_protocol);
    println!("Records:      {}", stats.record_count);
    println!("Entity types: {}", stats.type_count);
    println!("\nTop entity types:");
    for t in &stats.top_types {
        println!("  {:6}  {}", t.count, t.name);
    }
    if !stats.errors.is_empty() {
        println!("\nErrors: {}", stats.errors.len());
    }
}

fn print_analysis(result: &steprs::AnalysisResult) {
    print_stats(&result.stats);
    println!(
        "\nB-rep: {} solid(s), {} face(s), {} adjacency pair(s)",
        result.brep.solid_count, result.brep.face_count, result.brep.adjacency_count
    );
    println!(
        "Coaxial holes: {} (detect_coaxial_holes)",
        result.coaxial_holes.len()
    );
    for (i, f) in result.coaxial_holes.iter().enumerate() {
        println!("  [{}] {} — {:?}", i + 1, f.label, f.kind);
        if let Some(r) = f.radius {
            println!("       radius: {r:.3} mm");
        }
        if let Some(d) = f.depth {
            println!("       depth:  {d:.3} mm");
        }
        println!("       face_ids: {:?}", f.face_ids);
    }
    if result.stats.parse_errors > 0 {
        println!(
            "\nParse: {} non-fatal entity error(s)",
            result.stats.parse_errors
        );
    }
}
