mod common;
mod fixing;


use fixing::list_roots;

use crate::fixing::fix_loader;


fn main() {
    let mnt_dir = "/mnt/sysimage";
    
    let root = list_roots().unwrap().pop().unwrap();
    fix_loader(root.main_dev, root.subvol, mnt_dir).unwrap();
}
