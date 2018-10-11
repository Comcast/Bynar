# Disk Manager
This binary handles adding and removing disks from a server.  It uses
protobuf serialization to allow RPC usage. Please check the
[api crate](https://github.com/Comcast/Bynar/tree/master/api) for more information or the
[client crate](https://github.com/Comcast/Bynar/tree/master/client).

## Configuration:
1. Create your configuration file.  The utility takes json config
`/etc/bynar/disk-manager.json` file. This file should be deployed  
when the Bynar package is installed. The vault_* options are optional
but recommended.  When enabled the disk-manager upon starting will save
the generated public key to vault under `/bynar/{hostname}.pem`.  Any clients
wanting to connect to it will need to contact vault first.  If vault is
not enabled it will save the public key to /etc/bynar/.
```
{
  "backend": "ceph",
  "vault_endpoint": "https://my_vault:8888",
  "vault_token": "token_98706420"
}
```
Bynar that runs on Ceph, should have a ceph.json file to describe it. This tells 
where to look for ceph configuration, user details etc.
`/etc/bynar/ceph.json` file:
```
{
  "config_file": "/etc/ceph/ceph.conf",
  "user_id": "admin"
}
```
