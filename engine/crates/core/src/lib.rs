//! cutroom-core: the word-anchored edit model shared by every cutroom stage.
//!
//! * [`model`]   the raw recording flattened onto one source timeline (camera, screen, mic, words)
//! * [`spec`]    the YAML edit spec an agent writes (cuts, chapters, beats), anchored to words
//! * [`timemap`] source -> output time after cuts and inserted cards, frame-exact
//! * [`anchor`]  `w123`, `"a phrase"#2`, `s:754.2`, `o:61.5` resolution
//! * [`resolve`] spec + source -> [`resolve::Plan`] (everything in output time)
//! * [`compose`] what each output instant shows; shared by the renderer and the checker
//! * [`lint`]    the checks behind `cutroom check`

pub mod anchor;
pub mod compose;
pub mod lint;
pub mod model;
pub mod resolve;
pub mod spec;
pub mod timemap;

pub use model::{Clip, Media, Scene, Source, Track, Word};
pub use resolve::{Plan, ResolvedBeat, ResolvedChapter};
pub use spec::{BeatKind, Lane, LayoutMode, Spec};
