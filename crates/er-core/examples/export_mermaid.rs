fn main() {
    let src = include_str!("../../../fixtures/infor_hms_sample.mmd");
    let d = er_core::import_mermaid(src).expect("import");
    print!("{}", er_core::export_mermaid(&d));
}
