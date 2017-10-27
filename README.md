# Bynar
<!--<img src="">-->
Warehouse scale server repair, more benign than borg.

----

Bynar is an open source system for automating server maintenance
across the datacenter.  Bynar builds upon many years of experience
automating the drudgery of server repair. The goal is to have the
datacenter maintain itself.  Large clusters these days require
lots of maintenance.  [Cassandra], [Ceph], [Gluster], [Hadoop] and others
all require quick replacement of server parts as they break down or the cluster
becomes degraded.  The problem is that as your cluster grows you generally need to have more
people to maintain them.  Bynar hopes to break this cycle and
free your time up so your clusters can scale to ever greater sizes
without requiring more people to maintain them.  

The project is divided into different binaries that all communicate over protobuf:
1. disk-manager: This program handles adding and removing of disks from a server
2. dead-disk-detector:  This program handles detection of failed hard drives, files a ticket
for a datacenter technician to replace the drive, waits for resolution of the ticket and
then makes an API call to `disk-manager` to add the new disk back into the server.
3. client: Enables you to manually make API calls against `disk-manager`


----

## To start using Bynar

### Configuration:
1. Create your configuration file.  The utility takes json config
information.  Edit the `/etc/bynar/config.json` file to configure it.
An optional proxy field can be configured to send JIRA REST API requests through.
The slack_* fields are optional.  They will allow Bynar to send alerts to a
channel while it's performing maintenance.
Fields for this file are:
```
{
 "db_location": "/etc/bynar/disks.sqlite3",
 "proxy": "https://my.proxy",
 "manager_host": "localhost",
 "manager_port": 5555,
 "slack_webhook": "https://hooks.slack.com/services/ID",
 "slack_channel": "#my-channel",
 "slack_botname": "my-bot",
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
### Directory layout:
1. Top level is the dead disk detector
2. api is the protobuf api create
3. disk-manager is the service that handles adding and removing disks
4. client is the cli client to make RPC calls to disk manager or dead disk detector
5. parallel-deploy is a utility to quickly deploy a ceph osd cluster in parallel.  
Deploy 1000 osds in the time it takes to deploy 1 manually.

### Launch the program
1. After building bynar from source or downloading prebuilt packages
launch `disk-manager`, `dead-disk-detector` on every server you want
maintained.

## To start developing Bynar

This [community repository] hosts all information about
building Bynar from source, how to contribute code
and documentation, who to contact about what, etc.

If you want to build Bynar:

##### You have a working [Rust environment].

```
$ curl https://sh.rustup.rs -sSf | sh
$ cargo build --release
```
#### Dependencies:
1. libzmq3-dev  4.1 or higher
2. protobuf  2.5 or higher
3. librados  # ceph jewel or higher
4. libatasmart
5. openssl-dev


## Support

If you need support, start by checking the [issues] page.
If that doesn't answer your questions, or if you think you found a bug,
please [file an issue].

That said, if you have questions, reach out to us
[communication].

[Cassandra]: http://cassandra.apache.org/
[Ceph]: http://docs.ceph.com/docs/master/
[Hadoop]: http://hadoop.apache.org/
[Gluster]: https://www.gluster.org/
[communication]: https://github.com/cholcombe973/bynar/blob/master/communication.md
[community repository]: https://github.com/cholcombe973/bynar
[developer's documentation]: https://github.com/cholcombe973/blob/master/devel.md
[file an issue]: https://github.com/cholcombe973/bynar/issues/new
[issues]: https://github.com/cholcombe973/bynar/issues
