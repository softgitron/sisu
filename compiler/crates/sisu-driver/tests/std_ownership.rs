//! The standard library through the ownership checker, every function of
//! it: programs take std's functions as checked (`trust_std`), so this is
//! where they are checked.

#[test]
fn every_std_function_passes_the_ownership_checker() {
    let mut db = sisu_driver::Db::new();
    db.set_file("main.sisu", "def main():\n    pass\n");
    let c = db.checked("main.sisu").expect("checked");
    assert!(!c.has_errors());
    let sir = sisu_lower::lower_every_body(&c.program, &c.typed, &c.map, sisu_lower::ContractLevel::All);
    assert!(sir.functions.len() > 300, "{} functions", sir.functions.len());
    let checked = sisu_borrowck::check(&sir, &sisu_borrowck::Options::default());
    let shown: Vec<String> =
        checked.findings.iter().map(|f| format!("{}: {:?}", f.function, f.msg)).collect();
    assert!(checked.findings.is_empty(), "{shown:#?}");
}
