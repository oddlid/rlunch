fn main() -> shadow_rs::SdResult<()> {
    #[cfg(feature = "bundled")]
    {
        minijinja_embed::embed_templates!("templates");
    }
    shadow_rs::new()
}
