//! Render a project folder without the CLI: render_dir <dir> <out.mp4> [jobs] [codec] [height] [seconds]
use cutroom_core::{compose::segments, model::Source, resolve::build, spec};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&a[1]);
    let mut loaded = spec::load(&dir.join("edit.yaml"))?;
    if let Some(h) = a.get(5) {
        loaded.spec.output.height = h.parse()?;
    }
    let src = Source::load(&loaded.source_path)?;
    let plan = build(&loaded, &src);
    let segs = segments(&plan, &src);
    let range = a.get(6).map(|s| (0.0, s.parse::<f64>().unwrap() * 1000.0));
    let opts = cutroom_render::RenderOpts {
        out: a[2].clone().into(),
        jobs: a.get(3).map(|s| s.parse().unwrap()).unwrap_or(8),
        codec: a.get(4).map(|s| s.parse().unwrap()).unwrap_or(cutroom_render::Codec::VtH264),
        range,
        audio: true,
        loudnorm: false,
        system_gain: 0.0,
        work: None,
        quiet: true,
        hw_share: std::env::var("HW").ok().map(|v| v.parse().unwrap()).unwrap_or(0.0),
    };
    let r = cutroom_render::render(&plan, &src, &segs, &dir, &opts)?;
    println!("{}", serde_json::to_string(&r)?);
    Ok(())
}
