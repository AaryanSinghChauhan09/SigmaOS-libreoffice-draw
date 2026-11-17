use sigmaos_libreoffice_draw::{DrawDocument, DrawError, Shape, ShapeStyle};
use std::path::Path;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  sigmaos-libreoffice-draw new FILE TITLE");
    eprintln!("  sigmaos-libreoffice-draw rect FILE X Y W H");
    eprintln!("  sigmaos-libreoffice-draw circle FILE CX CY R");
    eprintln!("  sigmaos-libreoffice-draw line FILE X1 Y1 X2 Y2");
    eprintln!("  sigmaos-libreoffice-draw text FILE X Y TEXT");
    eprintln!("  sigmaos-libreoffice-draw layer FILE NAME");
    eprintln!("  sigmaos-libreoffice-draw use FILE LAYER");
    eprintln!("  sigmaos-libreoffice-draw remove FILE INDEX");
    eprintln!("  sigmaos-libreoffice-draw info FILE");
    eprintln!("  sigmaos-libreoffice-draw export FILE OUT.svg [WIDTH HEIGHT]");
    std::process::exit(2);
}

fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}

fn num(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or_else(|_| fail(format!("invalid number: {s}")))
}

fn idx(s: &str) -> usize {
    s.parse::<usize>().unwrap_or_else(|_| fail(format!("invalid index: {s}")))
}

fn load(file: &str) -> DrawDocument {
    if !Path::new(file).exists() {
        fail(format!("file not found: {file}"));
    }
    DrawDocument::load(file).unwrap_or_else(|e| fail(e))
}

fn save(doc: &DrawDocument, file: &str) {
    doc.save(file).unwrap_or_else(|e: DrawError| fail(e));
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        usage();
    }
    let cmd = args[0].as_str();
    let file = args[1].as_str();

    match cmd {
        "new" => {
            let title = args[2..].join(" ");
            let doc = DrawDocument::new(&title);
            save(&doc, file);
            println!("created {file}");
        }
        "rect" | "circle" | "line" => {
            let need = if cmd == "rect" || cmd == "line" { 7 } else { 6 };
            if args.len() < need {
                usage();
            }
            let shape = match cmd {
                "rect" => Shape::Rectangle { x: num(&args[2]), y: num(&args[3]), width: num(&args[4]), height: num(&args[5]) },
                "circle" => Shape::Circle { cx: num(&args[2]), cy: num(&args[3]), radius: num(&args[4]) },
                _ => Shape::Line { x1: num(&args[2]), y1: num(&args[3]), x2: num(&args[4]), y2: num(&args[5]) },
            };
            let _ = args.get(6);
            let mut doc = load(file);
            doc.add_shape(shape);
            save(&doc, file);
            println!("added {cmd}");
        }
        "text" => {
            if args.len() < 6 {
                usage();
            }
            let shape = Shape::Text { x: num(&args[2]), y: num(&args[3]), content: args[4..].join(" ") };
            let mut doc = load(file);
            doc.add_shape(shape);
            save(&doc, file);
            println!("added text");
        }
        "layer" => {
            if args.len() < 3 {
                usage();
            }
            let mut doc = load(file);
            doc.add_layer(&args[2]).unwrap_or_else(|e| fail(e));
            save(&doc, file);
            println!("added layer {}", args[2]);
        }
        "use" => {
            if args.len() < 3 {
                usage();
            }
            let mut doc = load(file);
            doc.set_active_layer(&args[2]).unwrap_or_else(|e| fail(e));
            save(&doc, file);
            println!("active layer {}", args[2]);
        }
        "remove" => {
            if args.len() < 3 {
                usage();
            }
            let mut doc = load(file);
            doc.remove_shape(idx(&args[2])).unwrap_or_else(|e| fail(e));
            save(&doc, file);
            println!("removed shape {}", args[2]);
        }
        "info" => {
            let doc = load(file);
            println!("{} ({} shapes, active layer {})", doc.title, doc.shape_count(), doc.active_layer());
            for (i, l) in doc.layers().iter().enumerate() {
                println!("  layer {}: {} ({} shapes)", i, l, doc.shapes_in_layer(l));
            }
            if let Some((x, y, w, h)) = doc.bounds() {
                println!("bounds: x={} y={} w={} h={}", x, y, w, h);
            }
            println!("total area: {}", doc.total_area());
        }
        "export" => {
            if args.len() < 3 {
                usage();
            }
            let doc = load(file);
            let w = args.get(3).map(|s| num(s)).unwrap_or(800.0);
            let h = args.get(4).map(|s| num(s)).unwrap_or(600.0);
            let svg = doc.to_svg(w, h).unwrap_or_else(|e: DrawError| fail(e));
            std::fs::write(&args[2], svg).unwrap_or_else(|e| fail(e));
            println!("exported {}", args[2]);
        }
        _ => usage(),
    }
}
