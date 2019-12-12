# Changelog

## [0.1.7] - 2019-12-12

### Changed

- [Use the new blkid function get_tag_value which is safer than do_probe][pr#103]
- [Cargo clippy and fmt fixes][pr#104]

### Added

- [Made Bynar main program into a daemon process when passed the --daemon flag.  Current signals handled are SIGINT, SIGCHLD, SIGHUP, and SIGTERM, the rest remain with default behavior][pr#105]

## [0.1.6] - 2019-11-22

### Changed

- [Updates to the crate versions][pr#88]
- [Revert ZMQ to ~0.8 to retain CurveKeyPair representation][pr#89]
- [Change filter_disks so it filters out /boot and /boot/efi partitions and disks][pr#90]
- [Updated pnet and lock file][pr#91]
- [Upgrade ceph crate to latest version][pr#94]
- [Replace ceph_safe_disk crate with Ceph command call osd safe-to-destroy][pr#95]
- [Change ceph backend remove_disk helper functions to follow proper removal procedure][pr#96]
- [Update dependencies for the debian][pr#100]


### Added

- [Added smartctl health scan check][pr#93]
- [Add a special case for Disk type block devices so they only undergo Scan transition][pr#97]
- [Add unmounts and mounts to functions and clean up temporary mounts][pr#98]
- [Add Attribute to CephConfig to filter out non-OSD devices and add Protobuf message type to handle skipped disk behavior][pr#100]
