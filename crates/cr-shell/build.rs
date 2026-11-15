// The built site (ui/dist, written by ui/tools/build.mjs and committed) goes into the binary, so a node that installs
// this cell from its commit serves the same bundles, by the same digests, with nothing else to copy.
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut es: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    es.sort();
    for p in es {
        if p.is_dir() {
            walk(root, &p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    let mut files = vec![];
    walk(&dist, &dist, &mut files);
    let mut s = String::from("pub const SITE: &[(&str, &[u8])] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(&dist).unwrap().to_string_lossy().replace('\\', "/");
        writeln!(s, "    ({rel:?}, include_bytes!({:?})),", f.canonicalize().unwrap().display().to_string()).unwrap();
    }
    s.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("site.rs"), s).unwrap();
}
