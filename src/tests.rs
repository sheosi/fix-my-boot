
#[test]
fn list_roots_test() {
    let roots = crate::list_roots("/mnt/test").unwrap();
    println!("{:?}", roots);
    assert!(roots.len() > 0);
}
