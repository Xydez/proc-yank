fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = embed_manifest::new_manifest("Io.Xydez.Textreactor");

        embed_manifest::embed_manifest(manifest).expect("unable to embed manifest");
    }

    // embed_manifest(new_manifest())
    println!("cargo:rerun-if-changed=build.rs");
}
