use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use crate::common::{Distribution, CallError};
use crate::fixing::Error;

use regex::Regex;
use serde::Deserialize;
use sys_mount::{Mount, MountFlags, SupportedFilesystems, Unmount,  UnmountDrop, UnmountFlags};

#[derive(Debug)]
pub struct RootData {
    pub main_dev: PathBuf,
    pub subvol: Option<String>,
    pub hostname: Option<String>,
    pub distro: Distribution,
}

#[derive(Debug, Deserialize)]
struct LsBlk {
    blockdevices: Vec<LsBlkBlockDevice>,
}

#[derive(Debug, Deserialize)]
struct LsBlkBlockDevice {
    name: String,

    #[serde(default)]
    fstype: Option<String>,

    #[serde(default)]
    uuid: Option<String>,

    #[serde(default)]
    children: Option<Vec<LsBlkBlockDevice>>,
}

const DISTRO_NAME_REGEX: &str = r"^NAME=(.*)$";
const DISTRO_VERSION_REGEX: &str = r"^VERSION=(.*)$";

pub fn list_roots<P>(mnt_point: P) -> Result<Vec<RootData>, Error> where P:AsRef<Path> {
    let out: LsBlk = 
        serde_json::from_str( 
            str::from_utf8(
                &CallError::from_output(
                    Command::new("/usr/bin/lsblk")
                        .arg("-fJ")
                        .output()
                )?.stdout
            ).unwrap()
        ).unwrap();

    let roots = out.blockdevices.into_iter()
            .filter(is_device)
            .map(get_children)
            .flatten()
            .map(|d| examine_disk(d, &mnt_point))
            .filter(|d|if let Ok(d2) = d {d2.system.is_some()} else{false})
            .map(|d| d.unwrap())
            .map(transform_to_data)
            .collect();

    

    Ok(roots)
}

fn is_device(blk: &LsBlkBlockDevice) -> bool {
    if blk.fstype.is_none() && blk.children.is_none() {
        false // Things like zram
    }
    else if let Some(fstype) = &blk.fstype {
        !matches!(fstype.as_str(), "squashfs")
    } 
    else {true} // Drives like /dev/sda
}

fn get_children(blk: LsBlkBlockDevice) -> Vec<LsBlkBlockDevice> {
    if let Some(children) = blk.children {
        children
    }
    else {
        vec![blk]
    }
    
}

fn mount<P1, P2>(dev: P1, mnt_point: P2) -> Result<UnmountDrop<Mount>, std::io::Error> where
    P1: AsRef<Path>,
    P2: AsRef<Path> {
            
    // Fetch a list of supported file systems.
    // When mounting, a file system will be selected from this.
    let supported = SupportedFilesystems::new().unwrap();
    println!("mnt_point: {:?}", mnt_point.as_ref());
    println!("dev: {:?}", dev.as_ref());

    // Attempt to mount the src device to the dest directory.
    let mount_result = Mount::new(
        dev,
        mnt_point,
        &supported,
        MountFlags::empty(),
        None
    )?;

    Ok(mount_result.into_unmount_drop(UnmountFlags::DETACH))
}


struct SystemData {
    pub subvol: Option<String>,
    pub hostname: String,
    pub distro: Distribution,
}

struct DiskData {
    pub system: Option<SystemData>,    
    pub blk: LsBlkBlockDevice
}

fn examine_disk<P>(blk: LsBlkBlockDevice, mnt_point: P) -> Result<DiskData, io::Error> where P: AsRef<Path> {
    let dev = Path::new("/dev").join(&blk.name);
    let mnt_point = mnt_point.as_ref();

    // As soon as this guard is dropped, the mount is unmounted
    let _mnt_grd = mount(&dev, mnt_point).unwrap();

    // We'll determine if it is root by looking at fstab
    let mut is_root = mnt_point.join("etc/fstab").exists();
    let mut subvol = None;
    
    let is_btrfs = if let Some(fstype) = &blk.fstype {
        fstype.as_str() == "btrfs"
    }
    else {false};

    // Try to find a subvolume
    if !is_root && is_btrfs {
        for elem in fs::read_dir(mnt_point).unwrap().flatten() {            
            if let Ok(f_type) =elem.file_type() {
                if f_type.is_dir() && elem.path().join("etc/fstab").exists() {
                    is_root = true;
                    subvol = Some(elem.file_name().to_str().unwrap().to_string());
                    break;
                }
            }
        }
    }

    let system = if is_root {
        let root = if let Some(subvol) = &subvol {
            mnt_point.join(subvol)
        }
        else {
            mnt_point.to_path_buf()
        };

        let distro = {
            let os_rel_data = fs::read_to_string(
                root.join("etc/os-release")
            ).unwrap_or_else(|_| String::new());

            let distro_reg = Regex::new(DISTRO_NAME_REGEX).expect("Distro regex compilation failed");
            let version_reg = Regex::new(DISTRO_VERSION_REGEX).expect("Version regex compilation failed");

            fn extract_reg<'a>(reg: &Regex, input: &'a str) -> Option<&'a str> {
                reg.captures(input).and_then(|c|c.get(1).map(|m|m.as_str()))
            }

            let distro_name = 
                extract_reg(&distro_reg, &os_rel_data)
                .unwrap_or("Linux");

            let version = extract_reg(&version_reg, &os_rel_data);
            
            match distro_name {
                "Fedora Linux" => Distribution::Fedora/*(version.unwrap_or("").to_string())*/,
                _ => Distribution::Unknown
            }
        };

        let hostname = fs::read_to_string(
            root.join("etc/hostname")
        ).unwrap_or_else(|_| String::new());



        Some(SystemData {
            subvol,
            distro,
            hostname
        })
    }
    else {
        None
    };


    Ok(
        DiskData {
            system,
            blk
        }
    )
}

fn transform_to_data(dsk: DiskData) -> RootData {
    let system = dsk.system.expect("Got a non system disk, please report this");
    RootData {
        main_dev: Path::new(&dsk.blk.name).to_path_buf(),
        subvol: system.subvol,
        hostname: Some(system.hostname),
        distro: system.distro,
    }
}