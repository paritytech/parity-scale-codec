fn main() {
	if rustversion::cfg!(nightly) {
		println!("cargo:rustc-check-cfg=cfg(nightly)");
		println!("cargo:rustc-cfg=nightly");
	}
}
