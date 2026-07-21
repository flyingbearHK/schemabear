fn main() {
    let src = include_str!("../../../fixtures/infor_hms_sample.mmd");
    let mut d = er_core::import_mermaid(src).expect("import");
    d.name = "Infor HMS Sample".into();
    er_core::auto_layout(&mut d, true);
    print!("{}", er_core::export_dbml(&d));
}
