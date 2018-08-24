# Bynar
<img src="bynar.png" width=100>

Warehouse scale server repair, more benign than borg.

----

Bynar is an open source system for automating server maintenance
across the datacenter.  Bynar builds upon many years of experience
automating the drudgery of server repair. The goal is to have the
datacenter maintain itself.  Large clusters these days require
lots of maintenance.  [Cassandra], [Ceph], [Gluster], [Hadoop] and others
all require quick replacement of server parts as they break down or the cluster
can become degraded.  As your cluster grows, you generally need to have more
people to maintain them.  Bynar hopes to break this cycle and
free up your time so that your clusters can scale to ever greater sizes
without requiring more people to maintain them.  

The project is divided into different binaries that all communicate over protobuf:
1. disk-manager: This program handles adding and the removal of disks from a server
2. dead-disk-detector:  This program handles detection of failed hard drives, files a ticket
for a datacenter technician to replace the drive, waits for the resolution of the ticket and
then makes an API call to `disk-manager` to add the new disk back into the server.
3. client: Enables you to manually make API calls against `disk-manager`


----

## To start using Bynar

### Configuration:
1. Create your configuration file.  The utility takes json config
information.  Edit the `/etc/bynar/bynar.json` file to configure it.
The slack_* fields are optional.  They will allow Bynar to send alerts to a
channel while it's performing maintenance. JIRA is the only currently supported
back end ticketing system.  A plugin system allows for more back end support.  
An optional proxy field can be configured to send JIRA REST API requests through.
For extra security we highly recommend that you enable the vault integration.
The disk-manager sits on a port and if an attacker gains access to it they can
quickly wipe out your disks.  If you don't wish to enable vault integration
set the disk-manager up to only listen on a loopback port.
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
 "jira_ticket_assignee": "assignee_username",
 "vault_endpoint": "https://my_vault.com",
 "vault_token": "token_98706420"
}
```
### Directory layout:
1. Top level is the dead disk detector
2. api is the protobuf api create
3. disk-manager is the service that handles the adding and removal of disks
4. client is the cli client to make RPC calls to disk manager or dead disk
detector

### Launch the program
1. After building Bynar from source or downloading prebuilt packages
launch the `disk-manager`, `dead-disk-detector` service on every server you want
maintained.

## To start developing Bynar

This [community repository] hosts all information about
building Bynar from source, how to contribute code
and documentation, who to contact about what, etc.

If you want to build Bynar:

##### You have a working [Rust environment].

```
$ curl https://sh.rustup.rs -sSf | sh
$ rustup override set nightly
$ cargo build --release
```
#### Dependencies:
1. libzmq3-dev  4.1 or higher
2. protobuf  2.5 or higher
3. librados  # ceph jewel or higher
4. libatasmart
5. openssl-dev
6. libblkid-dev

## Hard Drive Workflow
Hard drives die all the time as part of the regular cycle of things in servers.  Bynar
can nearly completely automate that maintenance except for the actual replacing of
the drive.  The typical workflow by a human would look something like this:
1. Receive an alert about a drive failing
2. SSH over to the server to investigate.  Try to rule out obvious things
3. Conclude drive is dead and file a support ticket with the datacenter tech to remove it
   * Or file a ticket with HP/Dell/Cisco/Etc to replace the drive
4. Depending on the software running on top of this drive I may have to:
   * Inform the cluster that the drive is dead
   * Rebalance the data in the cluster
5. Wait for a replacement
6. After the drive is replaced inform the clusters that the drive is now back
in service and rebalance the data back onto the drive.

So how can Bynar help?  Well it can handle steps 1,2,3,4 and 6.  Nearly everything!
While it is replacing your drives it can also inform you over slack or other channels
to keep you in the loop.
The time saved here multplies with each piece of hardware replaced and now you 
can focus your time and energy on other things.  It's a positive snowball effect!

## Testing

Note that root permissions are required for integration testing.  The reason
is that the test functions will attempt to create loopback devices, mount them,
check their filesystems etc and all that requires root. The nightly compiler
is also required for testing because mocktopus makes use of features that 
haven't landed in stable yet.  Run: `sudo ~/.cargo/bin/cargo test -- --nocapture` to test.

## Support and Contributions

If you need support, start by checking the [issues] page.
If that doesn't answer your questions, or if you think you found a bug,
please [file an issue].

That said, if you have questions, reach out to us
[communication].

Want to contribute to Bynar? Awesome! Check out the [contributing](https://github.com/Comcast/Bynar/blob/master/Contributing.md) guide.

[Cassandra]: http://cassandra.apache.org/
[Ceph]: http://docs.ceph.com/docs/master/
[Hadoop]: http://hadoop.apache.org/
[Gluster]: https://www.gluster.org/
[communication]: https://github.com/Comcast/Bynar/issues/new
[community repository]: https://github.com/Comcast/Bynar
[file an issue]: https://github.com/Comcast/Bynar/issues/new
[issues]: https://github.com/Comcast/Bynar/issues
