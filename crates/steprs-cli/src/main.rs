use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use steprs::{analyze_step, parse_only, PipelineOptions};
use steprs::path::{PostOptions, PostProcessor};

#[derive(Parser)]
#[command(name = "steprs", version, about = "STEP file analyzer — steprs.dev")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Phase 0/1: header + record statistics
    Inspect {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Phases 0–4: full pipeline
        Analyze {
            path: PathBuf,
            #[arg(long)]
            json: bool,
            #[arg(long)]
            gcode: Option<PathBuf>,
            #[arg(long)]
            no_gcode: bool,
            #[arg(long, default_value = "fanuc")]
            post: String,
        },
    /// List MVP feature catalog
    Catalog,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Catalog => {
            println!("Manufacturing feature MVP catalog:\n");
            for kind in steprs_core::ManufacturingFeatureKind::MVP {
                println!("  • {} ({:?})", kind.label(), kind);
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
        Commands::Analyze {
            path,
            json,
            gcode,
            no_gcode,
            post,
        } => {
            let content = fs::read_to_string(&path)?;
            let processor = match post.as_str() {
                "haas" => PostProcessor::Haas,
                "grbl" => PostProcessor::Grbl,
                "iso" => PostProcessor::Iso,
                _ => PostProcessor::Fanuc,
            };
            let options = PipelineOptions {
                emit_gcode: !no_gcode,
                post: PostOptions {
                    processor,
                    ..Default::default()
                },
                ..Default::default()
            };

            let result = analyze_step(&content, &options)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                print_analysis(&result);
            }

            if let Some(out) = gcode {
                if let Some(code) = &result.gcode {
                    fs::write(&out, code)?;
                    println!("\nG-code written to {}", out.display());
                }
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
        "Features: {} total (holes: {}, pockets: {}, bosses: {}, cylindrical: {})",
        result.features.features.len(),
        result.features.summary.hole_count,
        result.features.summary.pocket_count,
        result.features.summary.boss_count,
        result.features.summary.cylindrical_face_count
    );
    if let Some(sim) = &result.stock_simulation {
        println!(
            "\nStock sim: {:.0} mm³ removed · gouge: {} · overcut: {}",
            sim.removed_volume_mm3, sim.gouge_cells, sim.overcut_cells
        );
    }

    if let Some(tp) = &result.toolpath {
        println!(
            "\nToolpath: {} segments · cut {:.1} mm · rapid {:.1} mm · ~{:.2} min",
            tp.stats.segment_count,
            tp.stats.estimated_cut_length_mm,
            tp.stats.estimated_rapid_length_mm,
            tp.stats.estimated_time_min
        );
    }

    for (i, f) in result.features.features.iter().enumerate() {
        println!(
            "  [{}] {} — {:?} (conf {:.0}%)",
            i + 1,
            f.label,
            f.kind,
            f.confidence * 100.0
        );
        if let Some(r) = f.radius {
            println!("       radius: {r:.3} mm");
        }
        if let Some(d) = f.depth {
            println!("       depth:  {d:.3} mm");
        }
    }
    if let Some(v) = &result.gcode_validation {
        println!(
            "\nG-code validation: {}",
            if v.valid { "PASS" } else { "FAIL" }
        );
        for e in &v.errors {
            println!("  error: {e}");
        }
        for w in &v.warnings {
            println!("  warn:  {w}");
        }
    }
    if result.gcode.is_some() {
        println!("\nG-code: generated (use --gcode out.nc to save)");
    }
    if result.stats.parse_errors > 0 {
        println!(
            "\nParse: {} non-fatal entity error(s)",
            result.stats.parse_errors
        );
    }
}
