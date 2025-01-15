use shadow_rs::{BuildPattern, ShadowBuilder};

fn main() {
    #[cfg(feature = "bundled")]
    {
        minijinja_embed::embed_templates!("templates");
    }
    ShadowBuilder::builder()
        .build_pattern(BuildPattern::RealTime)
        .build()
        .unwrap();
}
