# Fix my boot

## Cloning
This uses git [LFS](https://git-lfs.github.com/) which means it needs to be
cloned with that installed beforehand.

First install git LFS, for *Debian/Ubuntu* it is:

```shell
sudo apt install git-lfs
```

On *Fedora*:    

```shell
sudo dnf install git-lfs
```

## Description


How many times has your boot broken? It's something so common to happen and despite the fact it is, it requires several complex commands (like having to chroot into the setup) and changing the UEFI.

"Fix my boot" is a tool that will fix boot-related problems in Linux systems, namely:
* Grub not being present in the UEFI partition.
* Linux not having a bootloader entry.

Fix my boot will:
* Look for a partition that seems to contain a Linux system (looks for "/etc/fstab").
* Mount it and chroot into it.
* It will check if there's a UEFI entry for that loader.
* If it can't find anything it will add it itself.

## Runtime dependencies

In order to check the system "Fix my boot" needs some tools in the host system: "lsblk", "fdisk", "efibootmgr".

## Compatibility

Right now this is only compatible with UEFI systems.

### Distributions
* Fedora