pub mod list_roots;

use std::path::Path;
use std::process::Command;
use crate::common::{Distribution, CallError, ReinstallError};
use thiserror::Error;

pub use list_roots::list_roots;

use self::uefi::check_bootloader_present;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    CallError(#[from] CallError),

    #[error("Target is not supported yet")]
    UnsupportedDistribution
}

mod chroot {
    use std::fs::create_dir_all;
    use std::io::Read;
    use std::path::Path;
    use std::process::Command;

    use regex::Regex;

    use crate::{common::{CallError, Distribution}, fixing::Error};

    const BOOT_REG: &str = r"([UUIDa-fA-F0-9-=]+)\s+\\/boot\s+(?:[^ ]*)";
    const UUID_REG : &str = "UUID=([a-fA-F0-9-=]+)";

    pub fn prepare<P1, P2>(dev: P1, subvol: Option<String>, mnt_dir: P2) -> Result<Distribution, Error>  where P1:AsRef<Path>, P2:AsRef<Path> {
        let mnt_dir = mnt_dir.as_ref();
        let dev = dev.as_ref();
        
        if !mnt_dir.exists() {
            create_dir_all(mnt_dir).unwrap();
        }
        
        let mut mnt = Command::new("/usr/bin/mount");
            mnt.args(&[dev.to_str().unwrap(), mnt_dir.to_str().unwrap()]);
        
        if subvol.is_some() {
            mnt.args(&["-o", &format!("subvol={}", subvol.unwrap())]);
        }

        CallError::from_res(mnt.status())?;

        fn mount_bind<P1, P2>(path: P1, mnt_dir: P2) -> Result<(), CallError> where P1:AsRef<Path>, P2:AsRef<Path> {
            CallError::from_res(Command::new("/usr/bin/mount")
                .args(&["--bind", 
                    path.as_ref().to_str().unwrap(),
                    mnt_dir.as_ref().join(&path).to_str().unwrap()
                ]).status()
            )
        }

        ["/dev/", "/proc", "/run", "/sys"].iter().map(|p| mount_bind(p, mnt_dir)).collect::<Result<Vec<_>,_>>()?;

        // Look for boot
        let mut fstab = String::new();
        let err = std::fs::File::open(
            mnt_dir.join("etc/fstab")
        ).unwrap().read_to_string(&mut fstab).unwrap();

        let boot_reg = Regex::new(BOOT_REG).expect("Regex failed to compile");
        if let Some(captures) = boot_reg.captures(&fstab) {
            let path_data = captures.get(1).unwrap().as_str();
            let uuid_reg = Regex::new(UUID_REG).expect("Regex failed to compile");

            let boot_path = if let Some(uuid_cap) = uuid_reg.captures(path_data) {
                let uuid = uuid_cap.get(1).unwrap().as_str();
                Path::new("/dev/disk/by-uuid").join(uuid)
            } else {
                Path::new(path_data).to_path_buf()
            };

            CallError::from_res(
                Command::new("/usr/bin/mount")
                    .args(&[
                        boot_path.to_str().unwrap(),
                        mnt_dir.join("boot").to_str().unwrap()
                    ]).status()
            )?;
        }

        let d = Distribution::Fedora;
        Ok(d)
    }
}

mod uefi {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use crate::fixing::Error;
    use crate::common::{CallError, Distribution};

    use regex::Regex;

    const EFI_REG : &str = r"^([\\/a-z0-9]+)\s+(?:[0-9]+)\s+(?:[0-9]+)\s+(?:[0-9]+)\s+(?:[0-9.KMGT]+)\s+EFI System";

    pub fn add_new_entry<P>(name: &str, uefi_dev: P, path: &str) -> Result<(), Error> where P:AsRef<Path> {
        let uefi_dev = uefi_dev.as_ref().to_str().unwrap();
        let device = &uefi_dev[..uefi_dev.len() - 1];
        let partition = &uefi_dev[uefi_dev.len() - 1..];
        CallError::from_res(Command::new("/usr/bin/sudo").args(&[
            "efibootmgr", "-c", "-w", "-L", name, "-d", device, "-p", partition, "-l", path
        ]).status())?;
        Ok(())
    }

    pub fn look_for_dev() -> Result<PathBuf, Error> {
        let out = CallError::from_output(
            Command::new("/usr/bin/sudo")
            .args(&["fdisk", "-l"])
            .env("LANG", "C") // Force english output
            .output()
        )?;

        
        let efi_reg = Regex::new(EFI_REG).expect("Regex failed to compile");
        let out_str = String::from_utf8(out.stdout).unwrap();
        let cap = efi_reg.captures(&out_str).unwrap();
        let dev = cap.get(1).unwrap().as_str();
        Ok(Path::new(dev).to_path_buf())
    }

    pub fn uefi_load_path(distr: &Distribution) -> Result<&'static str, Error> {
        match distr {
            Distribution::Fedora => Ok("/EFI/fedora/shim.efi"),
            &Distribution::Unknown => Err(Error::UnsupportedDistribution),
        }
    }

    pub fn check_bootloader_present<P>(efi_path: P) -> Result<bool, Error> where P: AsRef<Path> {
        let out = CallError::from_output(
            Command::new("/usr/bin/sudo")
            .args(&["efibootmgr", "-v"])
            .env("LANG", "C") // Force english output
            .output()
        )?;

        let out_str = String::from_utf8(out.stdout).unwrap();
        let efi_path = efi_path.as_ref()
            .to_str()
            .unwrap()
            .to_ascii_uppercase()
            .replace("\\", "/");

        Ok(out_str.contains(&efi_path))
    }
}

fn run_reinstall<P>(mnt_dir: P, distr: &Distribution) -> Result<(), ReinstallError>  where P:AsRef<Path> {
    ReinstallError::from_res(
        Command::new("reinstall_bootloader").args(&[
            mnt_dir.as_ref().to_str().unwrap(),
            &distr.to_string()
        ]).status().unwrap()
    )
}

pub fn fix_loader<P1, P2>(dev: P1, subvol: Option<String>, mnt_dir: P2) -> Result<(), Error>  where P1: AsRef<Path>, P2: AsRef<Path> {
    let distro = chroot::prepare(dev, subvol, &mnt_dir).unwrap();
    let uefi_dev = uefi::look_for_dev().unwrap();
    let uefi_path = uefi::uefi_load_path(&distro)?;
    
    run_reinstall(mnt_dir, &distro).unwrap();

    if !check_bootloader_present(uefi_path)? {
        uefi::add_new_entry(distro.loader_name(), uefi_dev, uefi_path).unwrap();
    }
    Ok(())
}