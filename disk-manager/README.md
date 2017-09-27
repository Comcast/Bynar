# Disk Manager
This binary handles adding and removing disks from a server.  It uses
protobuf serialization to allow RPC usage. Please check the
[api crate](https://github.com/cholcombe973/ceph_dead_disk/tree/master/api) for more information or the
[client crate](https://github.com/cholcombe973/ceph_dead_disk/tree/master/client).

## Configuration:
1. Create your configuration file.  The utility takes json config
`/etc/ceph_dead_disk/ceph.json` file:
```
{
  "config_file": "/etc/ceph/ceph.conf",
  "user_id": "admin"
}
```
This tells the Ceph backend how to talk to Ceph.
