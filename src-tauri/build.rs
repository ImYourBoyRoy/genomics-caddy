use std::{env, fs, path::PathBuf};

fn main() {
    tauri_build::build();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        embed_windows_test_manifest();
    }
}

fn embed_windows_test_manifest() {
    // The Rust test harness otherwise starts without Common Controls v6, and
    // Windows exits with STATUS_ENTRYPOINT_NOT_FOUND before running any tests.
    // Apply this only to test targets so the app keeps Tauri's configured
    // manifest and resource file unchanged.
    const WINDOWS_TEST_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    let manifest_path = out_dir.join("windows-test.manifest");
    fs::write(&manifest_path, WINDOWS_TEST_MANIFEST)
        .expect("write the Windows test harness manifest");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}
