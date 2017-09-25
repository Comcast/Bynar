Dead disk management for Ceph and more.

Stage 1: Disk replacement automation pipeline. There's several steps that used to require a human that I would like to cut out:
1. Identify bad disks
2. File a ticket request with JIRA
3. Keep track of the file request in a local Sqlite3 database
4. Watch for JIRA ticket resolution
5. Insert the disk back into the Ceph cluster

Stage 2: OSD monitoring to restart osds that are kicked out of the tree for some reason.  This might be better broken out into another utility.

Stage 3: Protobuf RPC API to allow remote control of ceph servers.  This might
also need to be broken out to another utility.  An RPC api for this utility
though could prove useful.

## Dependencies:
1. libzmq3-dev
2. protobuf
3. librados
4. libatasmart
5. openssl-dev

## Configuration:
1. Create your configuration file.  The utility takes json config
information.  Edit the `/etc/ceph_dead_disk/config.json` file to configure it. 
An optional proxy field can be configured to send JIRA REST API requests through.
Fields for this file are:
```
{
 "backend": "Ceph",
 "db_location": "/etc/ceph_dead_disk/disks.sqlite3",
 "proxy": "https://my.proxy",
 "jira_user": "test_user",
 "jira_password": "user_password",
 "jira_host": "https://tickets.jira.com",
 "jira_ticket_assignee": "username",
 "jira_issue_type": "3",
 "jira_priority": "4",
 "jira_project_id": "MyProject",
 "jira_ticket_assignee": "assignee_username"
}
```
and also create your `/etc/ceph_dead_disk/ceph.json` file:
```
{
  "config_file": "/etc/ceph/ceph.conf",
  "user_id": "admin"
}
```
This tells the Ceph backend how to talk to Ceph.

## Usage:

## Directory layout:
1. Top level is the dead disk detector
2. api is the protobuf api create
3. disk-manager is the service that handles adding and removing disks
4. client is the cli client to make RPC calls to disk manager or dead disk detector

## TODO:
- [ ] LSI Raid integration
- [ ] HP Raid integration
- [ ] NVME integration
- [x] libatasmart integration
- [ ] raid slot detection
