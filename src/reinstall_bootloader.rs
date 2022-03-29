mod common;

use std::env;
use std::path::Path;
use std::os::unix::fs;
use std::process::{exit, Command};

use common::{Distribution, CallError, ReinstallError};


pub fn change_to_chroot<P1>(mnt_dir: P1) -> Result<(), ReinstallError> where P1: AsRef<Path> {
    let mnt_dir = mnt_dir.as_ref();
    fs::chroot(mnt_dir).unwrap();
    env::set_current_dir(mnt_dir).unwrap();
    Ok(())
}

pub fn reinstall(distro: &Distribution) -> Result<(), ReinstallError> {
    match distro {
        Distribution::Fedora => {
            CallError::from_res(Command::new("/usr/bin/dnf")
                .args(&["install", "-y", "grub2-efi", "shim"]).status()).map_err(|e|e.into())
        }
        Distribution::Unknown => {
            /*CallError::from_res(Command::new("/usr/bin/grub2-install")
                .args(&["--target=x86_64-efi", "/dev/sda"]).status()).map_err(|e|e.into())*/
            Err(ReinstallError::OtherError) // TODO: Give this a proper error
        }
    }        
}

pub fn do_main() -> Result<(), ReinstallError> {
    let args: Vec<String> = env::args().collect();
    change_to_chroot(Path::new(&args[1]))?;
    reinstall(&args[2].parse().unwrap())
}

fn main() {
    if let Err(e) = do_main() {
        exit(e as i32);
    }
}