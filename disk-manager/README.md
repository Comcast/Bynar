# Disk Manager
This binary handles adding and removing disks from a server.  It uses
protobuf serialization to allow RPC usage. Please check the
[api crate](https://github.com/Comcast/Bynar/tree/master/api) for more information or the
[client crate](https://github.com/Comcast/Bynar/tree/master/client).

## Configuration:
1. Create your configuration file.  The utility takes json config
`/etc/bynar/disk-manager.json` file:
```
{
  "backend": "ceph",
  "vault_endpoint": "https://my_vault.com",
  "vault_token": "token_98706420"
}
```
For the ceph backend create a ceph.json file to describe it:
`/etc/bynar/ceph.json` file:
```
{
  "config_file": "/etc/ceph/ceph.conf",
  "user_id": "admin"
}
```
This tells the Ceph backend how to talk to Ceph.
