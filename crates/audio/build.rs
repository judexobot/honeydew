fn main() {
    let root = "../../vendor/nuked-opn2";
    for file in ["ym3438.c", "ym3438.h", "honeydew.c", "compat/string.h"] {
        println!("cargo:rerun-if-changed={root}/{file}");
    }
    let mut build = cc::Build::new();
    build
        .file(format!("{root}/ym3438.c"))
        .file(format!("{root}/honeydew.c"))
        .include(root)
        .opt_level(3)
        .warnings(false);
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        build
            .flag("-ffreestanding")
            .include(format!("{root}/compat"));
    }
    build.compile("honeydew_opn2");
}
