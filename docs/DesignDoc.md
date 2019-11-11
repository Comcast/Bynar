# Revision History

| Name           | Date       | Reason for Change                                                                                                                                                    | Version |
| -------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| Michelle Zhong | 10/8/2019  | Outline the Document                                                                                                                                                 | 0.1     |
| Michelle Zhong | 10/9/2019  | Outline the Document Modules, fill in the API section, Config File section, start filling out the Backend Section                                                    | 0.2     |
| Michelle Zhong | 10/10/2019 | Reorganize Headers in API section, Fill out the Backend, add Database Schema, add Error Module, Host Information, Helper Library                                     | 0.3     |
| Michelle Zhong | 10/11/2019 | Update Database Schema, Add Client, Jira Modules, Database Logging Section                                                                                           | 0.4     |
| Michelle Zhong | 10/14/2019 | Start Updating the Disk Testing Section                                                                                                                              | 0.5     |
| Michelle Zhong | 10/16/2019 | Updated the Disk Testing Section, Add Hardware Testing Section, add in the main Bynar program section                                                                | 0.6     |
| Michelle Zhong | 10/17/2019 | Added Section to Backend Explaining the GPT crate and what modifications are needed to fix its problems                                                              | 0.7     |
| Michelle Zhong | 11/1/2019  | Added Section on Daemonizing Bynar, explaining the Signal Handling and process of making a program a daemon. Add section on detecting maintenance (single-user) mode | 0.8     |
| Michelle Zhong | 11/11/2019 | Added Section on Current Bugs + Feature Implementations                                                                                                              | 0.9     |

# Table of Contents

[Revision History 2](#revision-history)

[Table of Contents 3](#_Toc24367301)

[List of Current Bugs and Feature Implementations
6](#list-of-current-bugs-and-feature-implementations)

[Terms 6](#terms)

[Bugs 6](#bugs)

[Features 7](#features)

[Maintenance (Single-User) Mode 7](#maintenance-single-user-mode)

[Background 7](#background)

[Daemonizing Bynar 7](#daemonizing-bynar)

[Background 7](#background-1)

[What is a Daemon 7](#what-is-a-daemon)

[Why are we daemonizing Bynar? 8](#_Toc24367311)

[Notes and Signal Handling 8](#notes-and-signal-handling)

[Old Style 8](#old-style)

[How to Daemonize a Process 9](#how-to-daemonize-a-process)

[SysV (Traditional) daemon 9](#sysv-traditional-daemon)

[New-Style Daemons (Systemd) 10](#new-style-daemons-systemd)

[API 10](#api)

[Introduction 10](#introduction)

[Messages 10](#messages)

[Enums 10](#enums)

[Structs 11](#structs)

[Configuration Files 13](#configuration-files)

[Introduction 13](#introduction-1)

[List of Config Files 14](#list-of-config-files)

[Bynar JSON 14](#bynar-json)

[Ceph JSON 14](#ceph-json)

[Disk-Manager JSON 14](#disk-manager-json)

[Backend 15](#backend)

[Introduction 15](#introduction-2)

[GPT 15](#gpt)

[Backend Module 15](#backend-module)

[Enums 15](#enums-1)

[Interface 15](#interface)

[Ceph 16](#ceph)

[Structs 16](#structs-1)

[Helper Functions 20](#helper-functions)

[Database Schema 24](#database-schema)

[Introduction 24](#introduction-3)

[Postgres 24](#postgres)

[Schema 25](#schema)

[Database Logging 25](#database-logging)

[Introduction 25](#introduction-4)

[Logging 25](#logging)

[Enums 25](#enums-2)

[Structs 26](#structs-2)

[Interface and Helper Functions 29](#interface-and-helper-functions)

[Helper Functions 34](#helper-functions-1)

[Introduction 34](#introduction-5)

[Error Module 34](#error-module)

[Type 34](#type)

[Enums 34](#enums-3)

[Structs 35](#structs-3)

[Host Information 36](#host-information)

[Enums 36](#enums-4)

[Structs 36](#structs-4)

[Helper Functions 37](#helper-functions-2)

[Helper Module 38](#helper-module)

[Structs 38](#structs-5)

[Helper Functions 38](#helper-functions-3)

[Client 40](#client)

[Introduction 40](#introduction-6)

[Client Interface 40](#client-interface)

[Support Tickets 42](#support-tickets)

[Introduction 42](#introduction-7)

[JIRA Support 42](#jira-support)

[Disk Manager 42](#disk-manager)

[Introduction 42](#introduction-8)

[Disk Manager 42](#disk-manager-1)

[Structs 42](#structs-6)

[Functions 43](#functions)

[Disk Testing 45](#disk-testing)

[Introduction 45](#introduction-9)

[State Machine 45](#state-machine)

[Special Cases 46](#special-cases)

[Type 46](#type-1)

[Trait 46](#trait)

[Enums 46](#enums-5)

[Structs 47](#structs-7)

[Functions 54](#functions-1)

[Hardware Testing 57](#hardware-testing)

[Introduction 57](#introduction-10)

[Hardware Tests 57](#hardware-tests)

[Struct 57](#struct)

[Functions 57](#functions-2)

[Bynar 58](#bynar)

[Introduction 58](#introduction-11)

[Main Process Functions 58](#main-process-functions)

# List of Current Bugs and Feature Implementations

### Terms

#### Status

  - BL – Back logged.

  - D – Draft stage. Either the Bug or the solution (or both) has not
    been flushed out yet

  - WIP – Work in Progress. Some work has/is being done.

  - PR – Pull Request made, waiting for review/fixes/nitpicks etc

  - Complete – Pull Request merged, will be removed on next revision
    update

## Bugs

| Description                                                                                                     | Solution/Implementation and Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Status |
| --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Filter does not filter out /boot or /boot/efi. If on MBR partitioned disk it is not filtered out.               | Check the mountpoint for /, /boot, and /boot/efi and filter those devices/partitions out                                                                                                                                                                                                                                                                                                                                                                                                | WIP    |
| Disk-manager port always binds to 5555 regardless of input port value                                           | Change the function so it takes in the port input if added (Note that this will add some complexity, for if the disk-manager port and the bynar/bynar-client port values do not match then they will not be able to communicate)                                                                                                                                                                                                                                                        | BL     |
| State Machine does not check if the filesystem/device is unmounted before running fsck                          | This is a rather minor bug since fsck should abort if the filesystem is unmounted unless the command is forced. Just add a check before running any fsck, or check\_filesystem commands for if the device is mounted first                                                                                                                                                                                                                                                              | BL     |
| Smart Health Check currently only uses libata’s smart checks. This can fail if the device does not support ATA. | Run smartctl (smartmon) health checks. Yes this can ALSO have problems since not all devices support smartmon tools/have it enabled. This would also add a dependency on having smartmontools installed. Different error types can let the program know if the error is an ACTUAL error or if it is due to a lack of the needed program on the device, which can help with the check.                                                                                                   | BL     |
| Ceph\_safe\_disk crate has JSON deserialize error                                                               | This probably is due to ceph well, outputting bad JSON in its newer versions. Luckily, ceph has a function ceph osd safe-to-destroy \<id\> that can be run now on Luminous+ versions (Jewel we don’t know yet). This may require an upgrade in the ceph crate version. Regardless, if it fails in Jewel, it is probably best to default to Not Safe to remove, so a manual check can be made                                                                                            | BL     |
| Ceph Journals should not ever get mounted (that includes their disks)                                           | We’ll need to figure out some special case for Ceph Journals, which still need to and can be scanned by smartmon tools, but cannot be mounted (and therefore cannot undergo read/write/filesystem corruption checks) so that they aren’t filed for replacement accidentally.                                                                                                                                                                                                            | D      |
| SCSIInfo does not get correctly sent to JIRA.                                                                   | Check\_all\_disks does happen to get the correct list of ScsiInfo objects. The problem is the BlockDevice object being used by JIRA uses a default ScsiInfo object reference which well, never gets replaced with the correct ScsiInfo object. So while the behavior in check\_all\_disks is correct since both are looped over, since the BlockDevice is never modified to use the correct ScsiInfo, well, it defaults to using the default. A fix will probably require a double loop | D      |

## Features

| Description                                                                          | Solution/Implementation                                                                                                                                                                                                                                                                                                                                                                                           | Status |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Maintenance Mode check. Bynar should not run while the system is in Maintenance mode | Current solution implementation is checking and creating a lock file when in maintenance mode, destroying it once maintenance mode is over. Another possible solution is run runlevel and check if it is N 1, and double check with systemctl get-default (if rescue.target). and prevent Bynar from running while that is true. Sample every run of Bynar? Then while maintenance mode == true check every hour? | WIP    |
| Daemonize Bynar                                                                      | Current solution uses the rust daemonize crate to make Bynar a daemon, still needs to implement signal handling                                                                                                                                                                                                                                                                                                   | WIP    |
| Better Loop checker for the State Machine                                            | ...Uh, this one needs more thought, especially on how much memory/processing we’re willing to spend...                                                                                                                                                                                                                                                                                                            | D      |
| ...                                                                                  |                                                                                                                                                                                                                                                                                                                                                                                                                   |        |

# Maintenance (Single-User) Mode

## Background

Maintenance, or single-user mode in linux distributions, is a boot mode
with only one user, and no network connection. What programs are
available in this mode differs per distribution. It is easy in Linux to
check if one is in this mode. Simply run either runlevel to get the run
level (N 1) is maintenance mode), or systemctl get-default, in which
rescue.target would be returned.

<https://www.ostechnix.com/check-runlevel-linux/>

# Daemonizing Bynar

## Background

### What is a Daemon

A daemon (not the helpful critters from Greek mythology but a program)
is a program that runs in the background. It performs tasks periodically
in a manner that usually goes unnoticed by users.

<span id="_Toc24367311" class="anchor"></span>Why are we daemonizing
Bynar?

Bynar can in fact, be run hourly as a CRON job. However, we would like
to run this in the background periodically, and while cron can run the
process periodically, making Bynar a daemon would make its tasks mostly
unnoticed by users.

## Notes and Signal Handling

A daemon should, in general, be able to handle various signals for
different reasons and pitfalls.

Old Vs New Style Daemonization:
<https://www.tecmint.com/systemd-replaces-init-in-linux/>

### Old Style

See <http://cjh.polyplex.org/software/daemon.pdf>

#### Background Job Control Write Checks

Some systems support 4.2BSD job control, and when a daemon attempts I/O
to their controlling terminal, it gets stopped if they were launched
from csh in the background. The true handling method is for the daemon
to disassociate from the controlling terminal, however the daemon might
want to perform some setup checks and output error messages beforehand.
While a background process cannot read from its controlling tty, output
can be performed with the SIGTTOU signal is ignored. If this is done,
usually it’s a good idea to ignore SIFTTIN and SIGNTSTP, however
ignoring SIFTTIN also causes all background attempts to read from the
control terminal to fail.

#### Close All Open File Descriptors

Do NOT leave any stray file descriptors open, and especially file
descriptors that are terminal devices. Terminal devices must be closed
to allow reset of the terminal state during logout.

#### Disassociate from Process Group and Controlling Terminal

If the daemon is launched during a login session, it inherits both the
controlling terminal and the process group of thast session or the job
within the session. While associated, the daemon is subject to all
terminal-generated signals such as SIGINT or SIGHUP. It is also subject
to job control terminal I/O restrictions. The daemon is also subject to
signals sent to the associated process group like kill.

You could ignore all signals, but then you can’t handle specific cases
and there are signals that cannot be ignored such as SIGKILL or SIGSTOP.

Disassociate instead from the controlling terminal and progress group.
If the system is 4.2BSD, TIOCNOTTY ioctl can be used as well as setpgrp.
If AT\&T UNIX, use setpgrp for both, but only if this is the first time
the process has called it (it is not already a group leader, aka process
group ID == process ID). In otherwords, fork first before calling
setpgrp. In general, the daemon should for and run in the child while
the parent immediately exits.

#### Do Not Reacquire a Controlling Terminal 

Once the daemon no longer has a controlling terminal, it can reacquire
one. It should not, as doing so will prevent other processes from
acquiring the terminal correctly as the controlling terminal. (NOTE this
is not a problem in 4.2BSD). Should the daemon reacquire the controlling
terminal, all login attempts for accounts with passwords will silently
fail without prompting for a password. Keyboard signals are not sent to
the processes spawned.

If using an AT\&T Unix system, a new controlling terminal is acquired
whenever a process group leader without one opens a terminal (that isn’t
also a control terminal), and once done there is no means to relinquish
it. To prevent this, fork again after calling setpgrp and ensure the
parent ignores the SIGHUP signal before forking the second child. The
second child (daemon) will then have no controller terminal and be in a
new process group immune to signals from the tty driver.

#### Do Not Hold Open TTY Files

Terminal state settings are only reset to the default state when the
LAST process having the terminal open finally closes it. If the daemon
has the terminal open continuously, then the last close never happens
and the settings are not reset. These files include stdin, stdout,
stderr, and /dev/console. Best practice is to log errors and status
messages to a disk file. If terminal logging is needed, then only hold
the terminal long enough to do a SINGLE logging transaction (its still a
window of time where a logout will not reset the terminal). Though since
this can STILL cause problems its best to just log to a disk file.

#### Change Current Directory to “/”

Every process has a current working directory that the kernel holds open
during the life of the process. If the process current directory is on a
mounted file system it cannot be dismounted by the admin without first
finding and killing the process. Daemons should adopt a current
directory NOT located on a mounted filesystem. The most reliable is the
root filesystem.

#### Reset the File Mode Creation Mask

A umask is associated with each process, which specifies how file
permissions are restricted for each file created by the process. Its
inherited from the parent process unless changed, and a daemon should
reset its umask to an appropriate value. (this is usually 0).

#### Any Inherited Attribute

In general, anything that can be inherited can cause problems, like the
nice priority value, the time left until an alarm signal, and the signal
mask and set of pending signals.

## How to Daemonize a Process

<https://stackoverflow.com/questions/17954432/creating-a-daemon-in-linux>

1.  Fork off the parent process and let it terminate (assuming forking
    was successful). The child process should now be running in the
    background

2.  Setsid and create a new session. The calling process should become
    the leader of the new sessions and the process group leader of the
    new process group. The process should now be detached from its
    controlling terminal (or CTTY)

3.  Catch signals (via ignoring or handling)

4.  Fork Again – and let the parent process terminate to ensure the
    session leading process is gone

5.  Chdir – change the working directory of the daemon

6.  Umask – change the file mode mask according to the daemon needs

7.  Close – close all open file descriptors that might be inherited from
    the parent process. ESPECIALLY stdin, stdout, stderr. If any of the
    file descriptors are terminal devices then they MUST be closed to
    allow reset of the terminal state during logout.

### SysV (Traditional) daemon

1.  Close all open file descriptors except stdin, stdout, stderr

2.  Reset all signal handlers to default

3.  Reset Sig Mask

4.  Sanitize environment block, remove or reset environmental variables
    that might break things

5.  Call fork

6.  Call setsid to detach from terminall

7.  Fork again so it cannot reacquire a terminal

8.  Call exit in the first child so only the second sticks around

9.  Connect /dev/null in daemon process to stdin/out/err

10. Reset umask to 0

11. Change current directory to root

12. Write the pid to a .pid file to ensure the daemon is only started
    once. (do this in a race free way)

13. Drop priviliges

14. Notify the original process that initialization is complete

15. Call exit in the original process

### New-Style Daemons (Systemd)

1.  If SIGTERM is received, shut down the daemon and exit cleanly

2.  If SIGHUP received, reload the config files (assuming there are
    config files)

3.  Provide a correct exit code from the main daemon process (this is
    used by init to detect service errors and problems). Recommended to
    use scheme defined
    <http://refspecs.linuxbase.org/LSB_3.1.1/LSB-Core-generic/LSB-Core-generic/iniscrptact.html>

4.  If possible (and applicable) expose daemon’s interface via D-Bus IPC
    system and grab a bus name as last step of init

5.  For integration in systemd, provice a .service unit file with
    information about start, stop, and maintaining the daemon.

6.  Rely on init system to limit access of daemon (yse systemd’s
    resource limit control, privilege dropping, etc.

7.  If D-Bus used, make daemon bus-activatable by supplying D-Bus
    service activation config file. This lets the daemon get started
    lazily on-demand, in parallel with other daemons, restarted on
    failure without losing bus requests etc.

8.  If daemon provides services to other local processes/remote clients
    via socket, it should be made socket-activatable following the
    scheme here
    <https://www.freedesktop.org/software/systemd/man/daemon.html#Activation>

9.  If applicable, the daemon should notify the init system about
    startup completion/status updates

10. Instead of using syslog(), log the error via fprintf(), which is
    forwarded to syslog by inity.

# API

## Introduction

This package uses Protobuf version 2 to create Messages that can be sent
over the network. Protobuf is a fast and small protocol for serializing
structs (or structured data). Serialized messages can be sent between
Sockets, unpackaged, and read easily and quickly. The protobuf package
automatically generates the rust code needed to create, modify, and
destroy Messages as well as their attributes.

## Messages

### Enums

#### DiskType

The type of disk or device

##### Enum Values

| Name         | Description                                             |
| ------------ | ------------------------------------------------------- |
| LOOPBACK     | Special loopback device                                 |
| LVM          | Logical Volume Device                                   |
| MDRAID       | Linux software RAID                                     |
| NVME         | Non-Volatile Memory Express, a logical device interface |
| RAM          | Ramdisk                                                 |
| ROTATIONAL   | Regular rotational device                               |
| SOLID\_STATE | SSD                                                     |
| VIRTUAL      | Virtual Disk                                            |
| UNKNOWN      | Unknown disk                                            |

#### ResultType

A result value

##### Enum Values

| Name | Description       |
| ---- | ----------------- |
| OK   | ok                |
| ERR  | There is an error |

#### Op

An operation on a disk

##### Enum Values

| Name              | Description                                             |
| ----------------- | ------------------------------------------------------- |
| Add               | Generic Add Disk command, returns an OpResult           |
| AddPartition      | Add a Partition Command, returns an OpResult            |
| List              | List the Disks, returns a list of Disks                 |
| Remove            | Remove a Disk, returns an OpResult                      |
| SafeToRemove      | Checks if a Disk is safe to remove, returns a bool      |
| GetCreatedTickets | list created tickets, returns a list of created tickets |

#### DatacenterOp

Datacenter API’s, these all require server\_id as a parameter for the
operation

##### Enum Values

| Name         | Description                                              |
| ------------ | -------------------------------------------------------- |
| GetDc        | Get ? Returns an OpStringResult                          |
| GetRack      | Get the rack of a server, returns an OpStringResult      |
| GetRow       | Get the row of a server, returns an OpStringResult       |
| GetElevation | Get the elevation of a server, returns an OpStringResult |

### Structs

#### Osd

A Ceph OSD object descriptor

##### Attributes

| Name          | Type             | Description                                 |
| ------------- | ---------------- | ------------------------------------------- |
| fsid          | Option\<String\> | OSD File System ID, if one exists           |
| id            | u64              | OSD ID number                               |
| block\_device | String           | Block Device of the OSD                     |
| journal       | Option\<String\> | Name of the Journal if the OSD has one set  |
| active        | bool             | Whether or not an OSD is active or a spare  |
| used\_space   | u64              | How much space in the OSD is currently used |
| total\_space  | u64              | Total space in the OSD                      |

#### Partition

A single partition descriptor

##### Attributes

| Name       | Type   | Description                                      |
| ---------- | ------ | ------------------------------------------------ |
| uuid       | String | The id of the partition                          |
| first\_lba | u64    | The first logical block address of the partition |
| last\_lba  | u64    | The last logical block address of the partition  |
| flags      | u64    | Flags associated with the partition              |
| name       | String | The name of the partition                        |

#### PartitionInfo

A list of Partitions

##### Attributes

| Name      | Type             | Description        |
| --------- | ---------------- | ------------------ |
| partition | Vec\<Partition\> | List of partitions |

#### Disk

A disk object descriptor

##### Attributes

| Name           | Type             | Description        |
| -------------- | ---------------- | ------------------ |
| type           | DiskType         | The type of disk   |
| dev\_path      | String           | ?? Device path?    |
| partitions     | PartitionInfo    | Disk partitions    |
| serial\_number | Option\<String\> | Disk serial number |

#### OpResult

A result of an Op message

##### Attributes

| Name       | Type             | Description                        |
| ---------- | ---------------- | ---------------------------------- |
| result     | ResultType       | Whether the result is ok or Error  |
| error\_msg | Option\<String\> | Error message if there is an error |

#### OpBoolResult

A boolean result of an Op message

##### Attributes

| Name       | Type             | Description                               |
| ---------- | ---------------- | ----------------------------------------- |
| result     | ResultType       | Whether Ok or Error                       |
| value      | Option\<bool\>   | A value is set if OK                      |
| error\_msg | Option\<String\> | Error message is set if there is an Error |

#### OpStringResult

A String result of an Op message

##### Attributes

| Name       | Type             | Description                               |
| ---------- | ---------------- | ----------------------------------------- |
| result     | ResultType       | Whether Ok or Error                       |
| value      | Option\<String\> | A value is set if OK                      |
| error\_msg | Option\<String\> | Error message is set if there is an Error |

#### JiraInfo

A Jira Ticket information descriptor

##### Attributes

| Name         | Type   | Description             |
| ------------ | ------ | ----------------------- |
| ticket\_id   | String | Ticket number           |
| server\_name | String | Name of the JIRA server |

#### OpJiraTicketsResult

A Jira ticket result

##### Attributes

| Name       | Type             | Description                               |
| ---------- | ---------------- | ----------------------------------------- |
| result     | ResultType       | Whether Ok or Error                       |
| value      | Option\<String\> | A value is set if OK                      |
| error\_msg | Option\<String\> | Error message is set if there is an Error |

#### DatacenterOperation

A Datacenter operation message

##### Attributes

| Name       | Type         | Description                            |
| ---------- | ------------ | -------------------------------------- |
| Op\_type   | DatacenterOp | The type of operation to be performed  |
| server\_id | String       | The ID of the server to be operated on |

#### Operation

A service operation that can be performed

##### Attributes

| Name             | Type             | Description                                                                   |
| ---------------- | ---------------- | ----------------------------------------------------------------------------- |
| Op\_type         | Op               | The operation type                                                            |
| disk             | Option\<String\> | The disk name, used for an Add or Remove                                      |
| simulate         | Option\<bool\>   | Whether the operation is a simulation, used for Add, Remove, and SafeToRemove |
| partition\_start | Option\<u64\>    | Optional field for AddPartition, start of a partition                         |
| partition\_end   | Option\<u64\>    | Optional field for AddPartition, end of a partition                           |
| partition\_name  | Option\<String\> | Optional field for AddPartition, partition name                               |
| osd\_id          | Option\<u64\>    | Optional Ceph related field, the id of an OSD                                 |
| replica\_set     | Vector\<String\> | Host:/dev/disk strings list for gluster replica sets                          |

# Configuration Files

## Introduction

Bynar uses a set of configuration files to configure different settings.
Bynar uses JSON as the format for its configuration files, as JSON files
are easily parsed, serialized, and deserialized using the Rust serde and
serde-json crates.

## List of Config Files

### Bynar JSON

This config file, bynar.json, is used to configure several different
settings, including a Slack webhook, JIRA support, Redfish access, Vault
password access, and Postgres database access

| Name                   | Description                               | Example Value                           |
| ---------------------- | ----------------------------------------- | --------------------------------------- |
| proxy                  | Proxy web server?                         | “https://my.proxy”                      |
| manager\_host          | The host ip of the bynar disk manager     | “localhost”                             |
| manager\_port          | The port of the Bynar disk manager        | 5555                                    |
| slack\_webhook         | Slack webhook to access Slack API         | "<https://hooks.slack.com/services/ID>" |
| slack\_channel         | Slack channel to post messages to         | “\#my-channel"                          |
| slack\_botname         | Name of the Bot to post messages under    | "my-bot"                                |
| jira\_user             | JIRA username to create tickets under     | “test\_user”                            |
| jira\_password         | JIRA password                             | “user\_pass”                            |
| jira\_host             | JIRA host to create tickets under         | “https://tickets.jira.com”              |
| jira\_issue\_type      | JIRA issue type name to create tickets of | “3”                                     |
| jira\_priority         | JIRA priority value of tickets created    | “4”                                     |
| jira\_project\_id      | JIRA project id to create tickets under   | “MyProject”                             |
| jira\_ticket\_assignee | User created JIRA tickets are assigned to | “assignee\_username”                    |
| redfish\_ip            | IP address of a Redfish instance          | “localhost”                             |
| redfish\_username      | Username to access Redfish instance       | “redfish\_user”                         |
| redfish\_password      | Password to access Redfish                | “redfish\_pass”                         |
| redfish\_port          | Port of the Redfish instance              | 4443                                    |
| vault\_endpoint        | Hashicorp vault endpoint                  | “https://my\_vault.com”                 |
| vault\_token           | Hashicorp vault token to access the vault | “token\_234464562”                      |
| database               | List of Database parameters               |                                         |
| database:username      | Username to access database with          | “postgres”                              |
| database:password      | Password to access database with          | “”                                      |
| database:port          | Port of the database                      | 5432                                    |
| database:dbname        | Name of the database                      | “bynar”                                 |
| database:endpoint      | Database endpoint                         | “some.endpoint”                         |

### Ceph JSON

This config file, ceph.json, is used to tell Bynar where the ceph.conf
file is, what user to use when running Ceph commands, and what journal
devices are known?

| Name                           | Description                            | Example Value         |
| ------------------------------ | -------------------------------------- | --------------------- |
| config\_file                   | The path to the ceph.conf file         | “/etc/ceph/ceph.conf” |
| user\_id                       | User to use when running Ceph commands | “admin”               |
| journal\_devices               | Journal device list                    |                       |
| journal\_devices:device        | Path of the device                     | “/dev/sda”            |
| journal\_devices:partition\_id | Partition ID number                    | 1                     |

### Disk-Manager JSON

This config file, disk-manager.json is used to tell Bynar what the
backend storage system is

| Name    | Description                            | Example Value |
| ------- | -------------------------------------- | ------------- |
| backend | The backend type of the storage system | "ceph”        |

# Backend

## Introduction

Different distributed storage clusters have different ways of adding and
removing disks, the backend module seeks to create an interface to the
different backends

## GPT

The GPT crate is used to parse the GUID Partition Table in a device.
Some modifications to the crate were necessary, as the num\_partitions
field in the GPT header actually indicates the total number of partition
entries. As such, the .partition() method returned all possible
partition entries, the actual partitions as well as empty entries. Due
to this, a change was made to skip all empty entries, which are zeroed
out, which lets GPT return the correct number of partitions.

## Backend Module

A Generic Module for interfacing with different storage backends

### Enums

#### BackendType

##### Enum Values

| Name    | Description                   |
| ------- | ----------------------------- |
| Ceph    | Ceph is the backend type      |
| Gluster | GlusterFS is the backend type |

##### Trait Implementations

###### FromStr

| Name      | Inputs   | Description                                                                                                             | Outputs                    |
| --------- | -------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| from\_str | s: \&str | Converts a string to a BackendType. Return Ok(BackendType) if successful or an Error if the string is not a BackendType | BynarResult\<BackendType\> |

###### Clone, Debug, Deserialize

### Interface

#### Backend

##### Trait Function Definition

<table>
<thead>
<tr class="header">
<th>Name</th>
<th>Inputs</th>
<th>Description</th>
<th>Outputs</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>add_disk</td>
<td><p>device: &amp;Path</p>
<p>id: Option&lt;u64&gt;</p>
<p>simulate: bool</p></td>
<td>Add a disk at path <em>device</em>, <em>id</em> an optional OSD id for Ceph clusters to ensure the OSD is set to that id, if <em>simulate</em> is passed no action is taken. Returns Ok(()) if successful or an Error if one occurs</td>
<td>BynarResult&lt;()&gt;</td>
</tr>
<tr class="even">
<td>remove_disk</td>
<td><p>device: &amp;Path</p>
<p>simulate: bool</p></td>
<td>Remove a disk at path <em>device</em> from a cluster. If <em>simulate</em> is passed no action is taken. Returns Ok(()) if successful or an Error if one occurs</td>
<td>BynarResult&lt;()&gt;</td>
</tr>
<tr class="odd">
<td>safe_to_remove</td>
<td><p>device: &amp;Path</p>
<p>simulate: bool</p></td>
<td>Check if safe to remove a disk from a cluster at path <em>device</em>. If <em>simulate</em> passed then return true. Returns Ok(true) if successful and safe, Ok(false) if successful and not safe to remove, or an Error if one occurs</td>
<td>BynarResult&lt;bool&gt;</td>
</tr>
</tbody>
</table>

##### Public Functions

<table>
<thead>
<tr class="header">
<th>Name</th>
<th>Inputs</th>
<th>Description</th>
<th>Outputs</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>load_backend</td>
<td><p>backend_type: &amp;BackendType</p>
<p>config_dir: Option&lt;&amp;Path&gt;</p></td>
<td>Given a BackendType, <em>backend_type,</em> and a config file directory from <em>config_dir</em>, return Ok(Backend) if successful or Error if one occurs.</td>
<td>BynarResult&lt;Box&lt;dyn Backend&gt;&gt;</td>
</tr>
</tbody>
</table>

## Ceph

The Ceph backend implementation

### Structs

#### CephBackend

This is a public struct object defining a Ceph cluster

##### Attributes

| Name            | Type        | Description                            |
| --------------- | ----------- | -------------------------------------- |
| cluster\_handle | Rados       | A handle to the ceph librados          |
| config          | CephConfig  | Handle for the Ceph Configuration File |
| version         | CephVersion | The Ceph Version                       |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(config_dir: Option&lt;&amp;Path&gt;) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Create a new CephBackend</p>
<p>PARAMETERS: config_dir – the directory of the ceph.json file or NONE if in the .config directory of the HOME directory</p>
<p>RETURNS: Ok(CephBackend) on success, Error otherwise</p>
<p>IMPLEMENTATION: Get the ceph.json file from the config_dir parameter. If successful, create the CephConfig object from the ceph.json file. Using the CephConfig object, connect to the specified Ceph instance using the specified user id to get the librados handle. Using the Rados handle, get the Ceph version string and convert it into a CephVersion object. If all steps are successful return a new CephBackend object with the CephConfig, Rados handle, and CephVersion.</p></td>
</tr>
<tr class="even">
<td><p>add_bluestore_osd(&amp;self, dev_path:&amp;Path, id:Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Add a bluestore OSD to the Ceph Cluster</p>
<p>PARAMETERS: dev_path – the device path of the OSD</p>
<p>id– the OSD id of the OSD to add</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Find a journal device that has enough free space? Create a new osd and get its osd_id (if id is not NONE then the new osd id should match id. Create an osd_fsid, and use it, the osd id, the device path, and the journal to create an lvm. Create a mount point path for the drive if necessary. Write the osd fsid to a file. Resolve the created lvm name to a true device path and chown it so ceph can use it. Symlink the lvm device name to the mount point’s /block, and if a journal device with enough space was found, symlink the journal to the mount point’s /block.wal and change the permissions so ceph can use it. Write activate monmap out by getting the map, and creating a file activate.monmap. Lookup the ceph user id and change all the permissions on the created files so ceph can use them. Create a ceph authorization entry, get the keyring created and save it. Format the osd with the osd filesystem. Use the ceph bluestore tool, and add the osd to the crush. Enable the osd, and then initialize the osd. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>add_filestore_osd(&amp;self, dev_path:&amp;Path, id:Option&lt;u64&gt;, simulate:bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Add a new /dev/ path as an osd, with xfs, for Jewel or earlier</p>
<p>PARAMETERS: dev_path – the device path of the OSD</p>
<p>id– the OSD id of the OSD to add</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Format the drive with the Xfs filesystem. Prove the drive by getting the device info and checking if it has a filesystem id. Create a new osd and get its id, which should be the same as the input id if one was input. Create the mount point path and mount the drive. Select a journal with enough space (if there is one, can be None). Format the osd with the osd filesystem. Create a ceph authorization entry, get the authorization key and save the keyring. Add the osd to the crush, add the osd to the fstab, then init the osd. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="even">
<td><p>change_permissions(&amp;self, paths: &amp;[&amp;Path], perms: &amp;Passwd) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: change permissions of many files at once</p>
<p>PARAMETERS: paths – the paths of the files to change the permissions of</p>
<p>perms – the group and owner permissions to change the file permissions to</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: loop through the paths and chown each path to the input permission values. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>create_lvm(&amp;self, osd_fsid: &amp;uuid::Uuid, new_osd_id: u64, dev_path: &amp;Path, journal_device: Option&lt;&amp;JournalDevice&gt;) -&gt; BynarResult&lt;(PathBuf, u64)&gt;</p>
<blockquote>
<p>DESCRIPTION: Create the lvm device and return the path and size of it</p>
<p>PARAMETERS: osd_fsid – the osd filesystem id</p>
<p>new_osd_id – the id of the osd</p>
<p>dev_path – the path to the device of the osd</p>
<p>journal_device – an optional journal device ? Dunno what it’s used for...</p>
<p>RETURNS: Ok(PathToLvm,Size) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: probe the device for its udev info. create a volume group name, and logical volume name, and use them to create the logical volume device name. Initialize a new LVM, and scan it. create the volume group on the LVM, then add the device path to the volume group by extending it and writing. create a linear logical volume in the volume group, create its tags. If all steps are successful return the path to the lvm device name and the volume group size, else it error’d out somewhere.</p></td>
</tr>
<tr class="even">
<td><p>create_lvm_tags(&amp;self, lv: &amp;LogicalVolume&lt;_,_&gt;, lv_dev_name: &amp;Path, osd_fsid: &amp;uuid::Uuid, new_osd_id:u64, info:&amp;block_utils::Device, journal_device:Option&lt;&amp;JournalDevice)-&gt;BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Add the lvm tags that ceph requires to identify the osd</p>
<p>PARAMETERS: lv – the logical volume</p>
<p>lv_dev_name – the path to the logical volume device</p>
<p>osd_fsid – the osd filesystem id</p>
<p>new_osd_id – the id of the osd</p>
<p>info – the device info</p>
<p>journal_device – an optional journal device ? Dunno what it’s used for...</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: create the lvm tags. If there is a journal device input, add a tag for the wal_device and add the wal_uuid. Once all tags are created add them to the logical volume. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>remove_bluestore_osd(&amp;self, dev_path:&amp;Path, simulate:bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Remove a bluestore OSD to the Ceph Cluster</p>
<p>PARAMETERS: dev_path – the device path of the OSD</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Initialize an lvm and scan it for volume groups and LVM metadata. Get the volume group that the device is associated with, if it cannot find the volume group, check if it is a filestore and if so fall back. otherwise, open the volume group and list all logical volumes in the volume group. List the tags to get the osd id and osd fsid. Set the osd as out, remove it from the crush, delete the authorization key, stop the osd, and remove it. Then, wipe the disk. remove all the logical volumes associated with the volume group, remove the volume group, and remove the physical volume and erase the physical volume. Then disable the osd. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="even">
<td><p>remove_filestore_osd(&amp;self, dev_path: &amp;Path, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Remove a bluestore OSD to the Ceph Cluster</p>
<p>PARAMETERS: dev_path – the device path of the OSD</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: get the mountpoint of the dev path and get the osd_id. Set the osd as out, remove it from the crush, delete the osd auth key, and remove the osd. Then, wipe the disk by erasing the block device. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>resolve_lvm_device(&amp;self, lv_dev_name: &amp;Path) -&gt; BynarResult&lt;PathBuf&gt;</p>
<p>DESCRIPTION: Resolve the lvm device name to an absolute path, since the lvm device name is a symlink, so it needs to be resolved to an absolute path to do anything with it.</p>
<blockquote>
<p>PARAMETERS: lv_dev_name – the device name of the lvm</p>
<p>RETURNS: Ok(Lvm Absolute Path) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: read the symlink. If it is a relative path, get its parent and the relative path to its parent, and canonicalize it, which returns the canonical, absolute form of a path with all intermediate components normalized and symbolic links resolved. If all steps are successful return the absolute path, else it error’d out somewhere.</p></td>
</tr>
<tr class="even">
<td><p>select_journal(&amp;self) -&gt; BynarResult&lt;Option&lt;JournalDevice&gt;&gt;</p>
<p>DESCRIPTION: Find a journal device that has enough free space if there is one</p>
<blockquote>
<p>PARAMETERS:</p>
<p>RETURNS: Ok(Some(JournalDevice)) or Ok(None) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: get the journal size from the Rados config. Convert it from MB to bytes. Get the journal devices from the ceph.json and sort them by the number of partitions. Iterate over the journal devices and remove the devices that are too small, and take the first journal with enough space. If all steps are successful, return Ok(Some(JournalWithEnoughSpace)) or Ok(None) if there are no journals with enough space, else it error’d out somewhere.</p></td>
</tr>
</tbody>
</table>

##### Trait Implementation

###### Backend

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>add_disk(&amp;self, device: &amp;Path, id: Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Add a disk to the Cluster</p>
<p>PARAMETERS: device – the device path of the disk to add</p>
<blockquote>
<p>id – an optional id to give the osd</p>
<p>simulate – if passed, skip the evaluation of this function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: If the Ceph Version is &gt;= Luminous, then run add_bluestore_osd. Otherwise, run add_filestore_osd. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="even">
<td><p>remove_disk(&amp;self, device:&amp;Path, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: remove a disk from the Cluster</p>
<p>PARAMETERS: device – the device path of the disk to add</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: check if the Ceph Version is &gt;= Luminous. If so, run remove_bluestore_osd. Otherwise, run remove_filestore_osd. If all steps are successful return (), else it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>safe_to_remove(&amp;self, _device:&amp;Path, _simulate:bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: check if a disk is safe to remove from the cluster</p>
<p>PARAMETERS: device – the unused device path of the disk to remove</p>
<p>simulate – if passed skip execution of the function</p>
<p>RETURNS: Ok(True) or Ok(False)on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Create a DiagMap and run an exhaustive check. If all steps are successful, then return true if the Status is Safe, return false if the Status is NonSafe or Unknown, otherwise the function error’d out somewhere.</p></td>
</tr>
</tbody>
</table>

#### JournalDevice

A Journal Device

##### Attributes

| Name            | Type                 | Description                                    |
| --------------- | -------------------- | ---------------------------------------------- |
| device          | PathBuf              | The device name? Device path???                |
| partition\_id   | Option\<u32\>        | The id of the partition                        |
| partition\_uuid | Option\<uuid::Uuid\> | The user? Unique? id of the partition          |
| num\_partitions | Option\<usize\>      | The number of partitions in the Journal Device |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>update_num_partitions(&amp;mut self) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Discover the number of partitions on the device and update the num_partitions field</p>
<p>PARAMETERS:</p>
<p>RETURNS: Ok(CephBackend) on success, Error otherwise</p>
<p>IMPLEMENTATION: use GPT to get the number of partitions from the partition table, and update the num_partitions field. If all steps are successful, then return (), else the function error’d out somewhere</p></td>
</tr>
</tbody>
</table>

##### Trait Implementation

###### Display

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>fmt(&amp;self, f: &amp;mut fmt::Formatter) -&gt; fmt::Result</p>
<p>DESCRIPTION: format the journal device for printing as a string/displaying as a string</p>
<p>PARAMETERS: f: some formatter</p>
<p>RETURNS: Ok(()) on success, fmt::Result error type otherwise</p>
<p>IMPLEMENTATION: if there is a partition_id, display the device and the id, otherwise just display the device.</p></td>
</tr>
</tbody>
</table>

###### Clone, Debug, Deserialize, PartialEq

#### CephConfig

The ceph configuration object descriptor

##### Attributes

| Name             | Type                           | Description                                                                                                                                            |
| ---------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| config\_file     | String                         | The location of the ceph.conf file                                                                                                                     |
| user\_id         | String                         | The cephx user to connect to the Ceph service with                                                                                                     |
| journal\_devices | Option\<Vec\<JournalDevice\>\> | The /dev/xxx devices to use for journal partitions. Bynar will create new partitions on these devices as needed if no journal\_partition\_id is given. |

##### Trait Implementation

###### Deserialize, Debug

### Helper Functions

<table>
<thead>
<tr class="header">
<th>Helper Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>choose_ceph_config(config_dir: Option&lt;&amp;Path&gt;) -&gt; BynarResult&lt;PathBuf&gt;</p>
<p>DESCRIPTION: get the path of the ceph.json file.</p>
<p>PARAMETERS: config_dir – an optional path to the configuration directory</p>
<p>RETURNS: Ok(ceph.json path) on success, Error otherwise</p>
<p>IMPLEMENTATION: check if a config_dir was provided. If so, check the directory for a ceph.json file. If a config_dir is not provided, check in the Home directory under the .config directory for the ceph.json file. If the function was successful return Ok(ceph.json path) else the function error’d somewhere.</p></td>
</tr>
<tr class="even">
<td><p>get_osd_id_from_path(path: &amp;Path) -&gt; BynarResult&lt;u64&gt;</p>
<p>DESCRIPTION: A fallback function to get the osd id from the mount path. Note, is not 100% accurate but will work for most cases unless the disk is mounted in the wrong location or is missing the osd id in the path name</p>
<p>PARAMETERS: path – the mount path</p>
<p>RETURNS: Ok(osd id) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the last part of the path (file or directory name). If successful, split the name by ‘-’, and the osd-id SHOULD be the second item in the list created by the split. If the function was successful return Ok(osd_id) else the function error’d somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>get_osd_id(path: &amp;Path, simulate: bool) -&gt; BynarResult&lt;u64&gt;</p>
<p>DESCRIPTION: Get the osd id from the whoami file in the osd mount directory</p>
<p>PARAMETERS: path – the osd mount directory</p>
<p>RETURNS: Ok(osd id) on success, Error otherwise</p>
<p>IMPLEMENTATION: make the path to the whoami file, and read the whoami file. Contained in the whoami file should be the osd_id, so convert that into a u64 and return it. if the function is successful return Ok(osd_id), else the function error’d somewhere</p></td>
</tr>
<tr class="even">
<td><p>save_keyring(osd_id: u64, key: &amp;str, uid: Option&lt;u32&gt;, gid: Option&lt;u32&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: save a Ceph authentication key to a keyring file (Note: as of now it also overwrites the keyring file every time....)</p>
<p>PARAMETERS: osd_id – the osd id</p>
<blockquote>
<p>key – the authentication key</p>
<p>uid – the uid of the user who will own the keyring file</p>
<p>gid – the gid of the group that will own the keyring file</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: convert the uid and guid into Uid and Gid types. Get the path to the base directory and check if it exists. If so, create the keyring file and write the key to the created file, and chown it to the uid and gid. If successful, return Ok(()), otherwise the function error’d out.</p></td>
</tr>
<tr class="odd">
<td><p>add_osd_to_fstab(device_info: &amp;block_utils::Device, osd_id: u64, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: add the osd to the file systems table (fstab)</p>
<p>PARAMETERS: device_info: device information gathered from udev</p>
<blockquote>
<p>osd_id – the osd id</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the default value of the fstab (default path is /etc/fstab). Create an entry for the fstab, filling in the attributes: the device id for the fs_spec, the mount point of the osd, the filesystem type, and the mount options, the dump, and fsck_order. Add the entry to the fstab. If the function is successful, return Ok(()), else the function error’d somewhere</p></td>
</tr>
<tr class="even">
<td><p>partition_in_use(partition_uuid: &amp;uuid::Uuid) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: Look through all the /var/lib/ceph/osd/ directories and check if there is a matching partition id to the input id.</p>
<p>PARAMETERS: partition_uuid – the uid of the partition to check</p>
<p>RETURNS: Ok(partition in use or not) on success, Error otherwise</p>
<p>IMPLEMENTATION: for each osd in the system, get the journal symlink and do a sanity check on the journal symlink. Get the metadata of the symlink and do another sanity check. resolve the symlink path to get the device and probe it. Get the partition uid from the device and compare to the input path. If the same, then return Ok(true), if not the same return Ok(false), otherwise it error’d</p></td>
</tr>
<tr class="odd">
<td><p>systemctl_disable(osd_id: u64, osd_uuid: &amp;uuid::Uuid, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: run the systemctl disable command on an osd</p>
<p>PARAMETERS: osd_id – the id of the osd</p>
<blockquote>
<p>osd_uuid – the user id? Of the osd</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the command arguments, and create a new Command to run the systemctl command. If the command is successful, return Ok(()), else it error’d</p></td>
</tr>
<tr class="even">
<td><p>systemctl_enable(osd_id: u64, osd_uuid: &amp;uuid::Uuid, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: run the systemctl enable command on an osd</p>
<p>PARAMETERS: osd_id – the id of the osd</p>
<blockquote>
<p>osd_uuid – the user id? Of the osd</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the command arguments, and create a new Command to run the systemctl command. If the command is successful, return Ok(()), else it error’d</p></td>
</tr>
<tr class="odd">
<td><p>systemctl_stop(osd_id: u64, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: run the systemctl disable command on an osd</p>
<p>PARAMETERS: osd_id – the id of the osd</p>
<blockquote>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the command arguments, and create a new Command to run the systemctl command. If the command is successful, return Ok(()), else it error’d</p></td>
</tr>
<tr class="even">
<td><p>setup_osd_init(osd_id: u64, osd_uuid: &amp;uuid::Uuid, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: initialize (start) the osd after having prepared the osd (it should be down and in) and be up and in once the function is run successfully</p>
<p>PARAMETERS: osd_id – the id of the osd</p>
<blockquote>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: check which daemon is running on the system to use the correct command. If the daemon is Systemd, use the systemctl command to start the osd and check the output. If the daemon is Upstart, then use the start command to start the osd and check the output. If the function is successful Ok(()) is returned, otherwise it error’d out somewhere.</p></td>
</tr>
<tr class="odd">
<td><p>settle_udev() -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: wait for udevd to create device nodes for all detected devices</p>
<p>PARAMETERS: NONE</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the udevadm command with the argument “settle”. If successful, return Ok(()), else error.</p></td>
</tr>
<tr class="even">
<td><p>ceph_mkfs(osd_id: u64, journal: Option&lt;&amp;JournalDevice&gt;, bluestore: bool, monmap: Option&lt;&amp;Path&gt;, osd_data: Option&lt;&amp;Path&gt;, osd_uuid: Option&lt;&amp;uuid::Uuid&gt;, user_id: Option&lt;&amp;str&gt;, group_id: Option&lt;&amp;str&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Run ceph-osd –mkfs and return the osd UUID</p>
<p>PARAMETERS: osd_id – the id of the osd</p>
<blockquote>
<p>journal – a JournalDevice if it is used by the OSD</p>
<p>bluestore – whether the OSD is a bluestore or filestore</p>
<p>monmap – optional path to the monmap</p>
<p>osd_data – optional path to the osd data directory</p>
<p>osd_uuid – optional user id of the osd?</p>
<p>user_id – the optional user id permissions of the OSD</p>
<p>group_id - the optional group id permissions of the OSD</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: create the arguments to the ceph-osd –mkfs command. Add more arguments depending on the contents of the input, and run the ceph-osd command. If successful, return Ok(()), else it error’d</p></td>
</tr>
<tr class="odd">
<td><p>ceph_bluestore_tool(device: &amp;Path, mount_path: &amp;Path, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Prime a bluestore osd, generating the content for an osd data directory that can start up a bluestore osd</p>
<p>PARAMETERS: device – the path to the osd device</p>
<blockquote>
<p>mount_path – the mount path of the osd</p>
<p>simulate – if passed, skip the execution of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: create the arguments of the ceph-bluestore-tool command. Run the command. If the command is successful, return Ok(()), else it error’d</p></td>
</tr>
<tr class="even">
<td><p>create_journal(name: &amp;str, size: u64, path: &amp;Path) -&gt; BynarResult&lt;(u32, uuid::Uuid)&gt;</p>
<p>DESCRIPTION: create a new ceph journal on a given device with the name and size in bytes</p>
<p>PARAMETERS: name – the name pf the ceph journal</p>
<blockquote>
<p>size – the size of the journal in bytes</p>
<p>path – the path of the journal</p>
</blockquote>
<p>RETURNS: Ok((partition id, partition group id)) on success, Error otherwise</p>
<p>IMPLEMENTATION: open the GPT (GUID partition table) in writable mode and inspect the path in the GPT. Add a new partition to the GPT of type CEPH JOURNAL and write it to the disk. update the partition cache and read it back into the GPT, then check if the partition was added to the GPT. If everything runs successfully return Ok(partition id, partition guid), otherwise it error’d</p></td>
</tr>
<tr class="odd">
<td><p>enough_free_space(device: &amp;Path, size: u64) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: Check if there is enough free space on the disk to fit a partition size request</p>
<p>PARAMETERS: device – the path to the osd device</p>
<blockquote>
<p>size – the size of the partition request</p>
</blockquote>
<p>RETURNS: Ok(is there enough space?) on success, Error otherwise</p>
<p>IMPLEMENTATION: open the GPT and check the device path. Find the free sectors on the dish, and for each pair of free sectors, check if there is enough space (if the length of the free sector &gt; the input size). If the function is successful, return Ok(true) if there is a sector with enough space, Ok(False) if there is no sector with enough space, otherwise there was an error</p></td>
</tr>
<tr class="even">
<td><p>evaluate_journal(journal: &amp;JournalDevice, journal_size: u64) -&gt; BynarResult&lt;JournalDevice&gt;</p>
<p>DESCRIPTION: Attempt to discover if there is a device in the journal, create journal partition if needed, and return a path to use for the journal</p>
<p>PARAMETERS: journal – the journal to evaluate</p>
<blockquote>
<p>journal_size – the size of the journal partition</p>
</blockquote>
<p>RETURNS: Ok(path to journal) on success, Error otherwise</p>
<p>IMPLEMENTATION: If the journal has a partition id, and a device, check if the partition exists and whether its in use by another osd. We can check using the GPT table, looping over the partitions to find the requested partition id, and check all the other osd’s for this partition id. If it is in use or there is no journal partition, create a new partition for the journal and update the number of partitions. If successful, return Ok(JournalDevice) with the updated partition values, otherwise it error’d somwhere.</p></td>
</tr>
<tr class="odd">
<td><p>remove_unused_journals(journals: &amp;[JournalDevice]) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Checks all osd drives on the system against the journals and delets all unused partitions. Note: unused</p>
<p>PARAMETERS: journals – the list of journals</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: For each journal in the list, open the GPT and check the disk at the journal device. get all of the partitions on the disk, and check if each partition is in use. If not, mark it as unused and save and update the partitions, and write all changes to the disk. If successful, return Ok(()), otherwise it error’d somewhere</p></td>
</tr>
<tr class="even">
<td><p>is_filestore(dev_path: &amp;Path) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: Checks if the specified OSD is a filestore</p>
<p>PARAMETERS: dev_path – the device path of the osd</p>
<p>RETURNS: Ok(is a filestore?) on success, Error otherwise</p>
<p>IMPLEMENTATION: Get the mount point from the device path. If there isn’t a mountpoint, create a temporary osd mount point and mount the device. Add type to the path and check if the path exists. If so, check if the contents of the file contain “filestore”. If the function is successful and “filestore” type is found, return Ok(true), if successful and “filestore” is NOT found, return Ok(false), else it error’d</p></td>
</tr>
<tr class="odd">
<td><p>update_partition_cache(device: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Linux specific ioctl to update the partition table cache</p>
<p>PARAMETERS: device – the device path</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Open the device and run blkrrpart. If successful return Ok(()), else it error’d</p></td>
</tr>
</tbody>
</table>

# Database Schema

## Introduction

Bynar should have a database to log changes, errors, and other
noteworthy messages. Currently Bynar only supports Postgres

## Postgres

In the dbschema folder, there is a bynar\_stats.sql file. You will need
to import this into your Postgres Bynar Database. To import, you can run
\\i \<path to file\> from inside the psql prompt, or copy paste.

### Schema

![](media/image1.png)

# Database Logging

## Introduction

Most database logging functions are in the in\_progress.rs file. This
file holds functions that log changes and other important messages to a
database. Currently it only handles Postgres database integration.

## Logging

### Enums

#### OperationType

##### Enum Values

| Name                  | Description                    |
| --------------------- | ------------------------------ |
| DiskAdd               | Add a disk                     |
| DiskReplace           | Replace a disk                 |
| DiskRemove            | Remove a Disk                  |
| WaitingForReplacement | Waiting for a Replacement Disk |
| Evaluation            | ???? Evaluate a disk?          |

##### Trait Implementations

###### Display

| Name | Inputs             | Description                                        | Outputs     |
| ---- | ------------------ | -------------------------------------------------- | ----------- |
| fmt  | f: \&mut Formatter | Converts an OperationType to a String for printing | fmt::Result |

###### Debug

#### OperationStatus

##### Enum Values

| Name       | Description                |
| ---------- | -------------------------- |
| Pending    | Operation waiting to start |
| InProgress | Operation is running       |
| Complete   | Operation has finished     |

##### Trait Implementations

###### Display

| Name | Inputs             | Description                                          | Outputs     |
| ---- | ------------------ | ---------------------------------------------------- | ----------- |
| fmt  | f: \&mut Formatter | Converts an OperationStatus to a String for printing | fmt::Result |

###### Debug

### Structs

#### DiskRepairTicket

A Disk Repair Ticket, a table entry?

##### Attributes

| Name         | Type   | Description                  |
| ------------ | ------ | ---------------------------- |
| ticket\_id   | String | Id number of the ticket      |
| device\_name | String | Name of the device to repair |
| device\_path | String | Path to the device to repair |

##### Trait Implementation

######  Debug

#### DiskPendingTicket

Table entry???

##### Attributes

| Name         | Type   | Description                       |
| ------------ | ------ | --------------------------------- |
| ticket\_id   | String | Id number of the ticket           |
| device\_name | String | Name of the device ???? Pending?  |
| device\_path | String | Path to the device ??? Pending?   |
| device\_id   | i32    | ID number of the device? Pending? |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(ticket_id: String, device_name: String, device_path: String, device_id: i32) -&gt; DiskPendingTicket</p>
<p>DESCRIPTION: create a new DiskPendingTicket</p>
<p>PARAMETERS: ticket_id – the id number of the ticket</p>
<blockquote>
<p>device_name – the name of the pending? device</p>
<p>device_path – the path of the pending? Device</p>
<p>device_id – the id of the pending? device</p>
</blockquote>
<p>RETURNS: DiskPendingTicket</p>
<p>IMPLEMENTATION: create a new DiskPendingTicket with the input parameters</p></td>
</tr>
</tbody>
</table>

##### Trait Implementation

###### Debug

#### HostDetailsMapping

Table entry?

##### Attributes

| Name                | Type | Description                       |
| ------------------- | ---- | --------------------------------- |
| entry\_id           | u32  | Entry number?                     |
| region\_id          | u32  | Region number                     |
| storage\_detail\_id | u32  | Storage detail relation number??? |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(entry_id: u32, region_id: u32, storage_detail_id: u32) -&gt; HostDetailsMapping</p>
<p>DESCRIPTION: Create a new HostDetailsMapping table entry</p>
<p>PARAMETERS: entry_id – the table entry number</p>
<blockquote>
<p>region_id – the region id number</p>
<p>storage_detail_id – the reference to the storage_detail entry ID</p>
</blockquote>
<p>RETURNS: HostDetailsMapping</p>
<p>IMPLEMENTATION: create a new HostDetailsMapping with the input parameters</p></td>
</tr>
</tbody>
</table>

##### Trait Implementation

###### Debug

#### OperationInfo

An entry for the Operations Table

##### Attributes

| Name           | Type                      | Description                                      |
| -------------- | ------------------------- | ------------------------------------------------ |
| operation\_id  | Option\<u32\>             | The operation id                                 |
| entry\_id      | u32                       | The table entry id                               |
| device\_id     | u32                       | The device id                                    |
| behalf\_of     | Option\<String\>          | On behalf of what user                           |
| reason         | Option\<String\>          | The reason for the operation                     |
| start\_time    | DateTime\<Utc\>           | The start time of the operation                  |
| snapshot\_time | DateTime\<Utc\>           | The time when taking a snapshot of the operation |
| done\_time     | Option\<DateTime\<Utc\>\> | When the operation was finished                  |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Implementation</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(entry_id: u32, device_id: u32) -&gt; OperationInfo</p>
<p>DESCRIPTION: Create a new OperationInfo with an entry_id and device_id</p>
<p>PARAMETERS: entry_id – the table entry id</p>
<blockquote>
<p>device_id – the id number of the device being operated on</p>
</blockquote>
<p>RETURNS: OperationInfo</p>
<p>IMPLEMENTATION: Create a new OperationInfo filled with the input parameters with all optional fields set to None and the start and snapshot times defaulted to the current timestamp.</p></td>
</tr>
<tr class="even">
<td><p>set_operation_id(&amp;mut self, op_id: u32)</p>
<blockquote>
<p>DESCRIPTION: set the operation id number</p>
<p>PARAMETERS: op_id – the operation id number</p>
<p>RETURNS: the OperationInfo with its operation id set to the input id number<br />
IMPLEMENTATION: set the value of the oepration_id to the input id</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>set_done_time(&amp;mut self, done_time: DateTime&lt;Utc&gt;)</p>
<blockquote>
<p>DESCRIPTION: set the completion time</p>
<p>PARAMETERS: done_time - the timestamp of when the operation finished</p>
<p>RETURNS: the OperationInfo with its done_time set to the input completion time</p>
<p>IMPLEMENTATION: set the value of done_time to the input done_time</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>set_snapshot_time(&amp;mut self, snapshot_time: DateTime&lt;Utc&gt;)</p>
<blockquote>
<p>DESCRIPTION: set the snapshot time</p>
<p>PARAMETERS: snapshot_time – the time of the latest snapshot of the operation</p>
<p>RETURNS: the OperationInfo with its snapshot_time set to the latest snapshot time<br />
IMPLEMENTATION: set the value of snapshot_time to the input snapshot time</p>
</blockquote></td>
</tr>
</tbody>
</table>

##### Trait Implementation

######  Debug

#### OperationDetail

An entry for the operation\_details table

##### Attributes

| Name           | Type                      | Description                               |
| -------------- | ------------------------- | ----------------------------------------- |
| op\_detail\_id | Option\<u32\>             | Operation detail entry id number          |
| operation\_id  | u32                       | Link to the operation id number           |
| op\_type       | OperationType             | The operation type                        |
| status         | OperationStatus           | Current status of the operation           |
| tracking\_id   | Option\<String\>          | The tracking id number of the operation   |
| start\_time    | DateTime\<Utc\>           | The start time of the operation           |
| snapshot\_time | DateTime\<Utc\>           | The last snapshot time of the operation   |
| done\_time     | Option\<DateTime\<Utc\>\> | The time when the operation was completed |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(operation_id: u32, op_type: OperationType) -&gt; OperationDetail</p>
<p>DESCRIPTION: Create a new OperationDetail with optional fields set to None and start and snapshot time attributes set to the current timestamp</p>
<p>PARAMETERS: operation_id – the reference to the operation table</p>
<blockquote>
<p>op_type – the operation type</p>
</blockquote>
<p>RETURNS: OperationDetail</p>
<p>IMPLEMENTATION: create a new OperationDetail and set all optional values to None, set the operation_id and op_type to the input values, and default start and snapshot times to the current timestamp</p></td>
</tr>
<tr class="even">
<td><p>set_operation_detail_id(&amp;mut self, op_detail_id: u32)</p>
<blockquote>
<p>DESCRIPTION: set the operation detail id<br />
PARAMETERS: op_detail_id – the entry number<br />
RETURNS: OperationDetail with the operation_detail_id set to the input<br />
IMPLEMENTATION: set the value of operation_detail_id to the input operation detail id</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>set_tracking_id(&amp;mut self, tracking_id: String)</p>
<blockquote>
<p>DESCRIPTION: set the tracking id<br />
PARAMETERS: tracking_id – the tracking id<br />
RETURNS: OperationDetail with the tracking_id set to the input value<br />
IMPLEMENTATION: set the value of tracking_id to the input tracking id</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>set_done_time(&amp;mut self, done_time: DateTime&lt;Utc&gt;)</p>
<blockquote>
<p>DESCRIPTION: set the done time<br />
PARAMETERS: done_time – the time of the operation completion<br />
RETURNS: OperationDetail with the done_time set to the input completion time<br />
IMPLEMENTATION: set the value of done_time to the input completion time</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>set_operation_status(&amp;mut self, status: OperationStatus)</p>
<blockquote>
<p>DESCRIPTION: set the operation status<br />
PARAMETERS: status – the current status of the operation<br />
RETURNS: OperationDetail with the status set to the input status<br />
IMPLEMENTATION: set the value of status to the input status value</p>
</blockquote></td>
</tr>
</tbody>
</table>

##### Trait Implementation

######  Debug

### Interface and Helper Functions

<table>
<thead>
<tr class="header">
<th>Helper Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>create_bd_connection_pool(db_config: &amp;DBConfig) -&gt; BynarResult&lt;Pool&lt;ConnectionManager&gt;&gt;</p>
<p>DESCRIPTION: Reads the config file to establish a pool of database connections</p>
<p>PARAMETERS: db_config – the database configuration</p>
<p>RETURNS: Ok(connectionManager pool) on success, Error otherwise</p>
<p>IMPLEMENTATION: Convert the password in the Config to a str, since that’s what Postgres expects. Set the connection parameters, and create a ConnectionManager with the parameters. Build a pool of 10 threads to the Postgres database. If successful, return Ok(Pool of connections to Postgres) otherwise error out</p></td>
</tr>
<tr class="even">
<td><p>get_connection_from_pool(pool: &amp;Pool&lt;ConnectionManager&gt;) -&gt; BynarResult&lt;PooledConnection&lt;ConnectionManager&gt;&gt;</p>
<p>DESCRIPTION: return one connection from the pool</p>
<p>PARAMETERS: pool – the pool of connections to the database</p>
<p>RETURNS: Ok(A single pooled connection) on success, Error otherwise</p>
<p>IMPLEMENTATION: run pool.get to get a free connection thread. If successful, return Ok(single connection to the database), otherwise error out</p></td>
</tr>
<tr class="odd">
<td><p>update_storage_info(s_info: &amp;MyHost, pool: &amp;Pool&lt;ConnectionManager&gt;) -&gt; BynarResult&lt;HostDetailsMapping&gt;</p>
<p>DESCRIPTION: update the storage info in the database, should be called when the Bynar daemon starts and checks if all steps in the function are successful</p>
<p>PARAMETERS: s_info - the current host information of the program</p>
<blockquote>
<p>pool – the pool of connections to the database</p>
</blockquote>
<p>RETURNS: Ok(host details mapping) on success, Error otherwise</p>
<p>IMPLEMENTATION: get a single connection to the database. extract the ip address from the host information. start a new Postgres transaction to update the storage information in the database. Register the ip to the process manager, update the region info, and update the storage details. commit the Postgres SQL requests and create a new HostDetailsMapping with the returned values from the transaction calls. Finish the transaction, and if successful, return Ok(host details mapping), otherwise error out.</p></td>
</tr>
<tr class="even">
<td><p>register_to_process_manager(conn: &amp;Transaction&lt;’_&gt;, ip: &amp;str) -&gt; BynarResult&lt;u32&gt;</p>
<blockquote>
<p>DESCRIPTION: stores the pid, ip of the system on which bynar is running to the database</p>
<p>PARAMETERS: conn – the transaction connection to the database</p>
<p>ip – the ip to store</p>
<p>RETURNS: the entry id of the transaction<br />
IMPLEMENTATION: get the process id. Create the statement with the pid and ip. Query the database with the statement. If there is a response, get the entry id and update the process_manager table with the idle status. If there is response, insert into the process_manager table the pid, ip, and the idle status, getting back the entry id. If successful, return Ok(entry_id), otherwise error out.</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>deregister_from_process_manager() -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: When implemented, should de-register the process from the database when the daemon exists?? Exits???<br />
PARAMETERS: N/A<br />
RETURNS: N/A<br />
IMPLEMENTATION: N/A</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>update_region(conn: &amp;Transaction&lt;’_&gt;, region: &amp;str) -&gt; BynarResult&lt;u32&gt;</p>
<blockquote>
<p>DESCRIPTION: checks for the region in the database, inserts if it does not exist and returns the region_id<br />
PARAMETERS: conn – the connection to the database for transactions<br />
RETURNS: Ok(region_id) on success, else Error<br />
IMPLEMENTATION: Query the database for the region name. If it exists, return Ok(region_id), if it doesn’t, insert the region into the database and get the region_id. If successful, return Ok(region_id), else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>update_storage_details(conn: &amp;Transaction&lt;’_&gt;, s_info: &amp;MyHost, region_id: u32) -&gt; BynarResult&lt;u32&gt;</p>
<blockquote>
<p>DESCRIPTION: update the storage details in the database and get the storage_detail_id<br />
PARAMETERS: conn – the connection to the database for transaction</p>
<p>s_info – the storage host information</p>
<p>region_id – the region id number in the database</p>
<p>RETURNS: Ok(storage_detail_id) if successful, else Error<br />
IMPLEMENTATION: query if the database has the input storagetype. If so, query if the specific details are already in the database. If not, insert the array_name and pool_name into the database. If successful, return Ok(storage_detail_id), else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>add_disk_detail(pool: &amp;Pool&lt;ConnectionManager&gt;, disk_info: &amp;mut BlockDevice) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Inserts disk information record into bynar.hardware and adds the device_database_id to the struct<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>disk_info – the BlockDevice info to query about and fill in</p>
<p>RETURNS: Ok(()) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the disk details. If a record of the disk doesn’t exist, insert the disk_info information into the database and get the device_database_id number. If the device exists in the database, check if it matches the input struct and get the device_database_id. If successful, return Ok(device_database_id), else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>add_or_update_operation(pool: &amp;Pool&lt;ConnectionManager&gt;, op_info: &amp;mut OperationInfo) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: inserts or updates the operation record. If a successful insert, the provided input op_info is modified. Errors if insert fails<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>op_info – the operation info</p>
<p>RETURNS: Ok(()) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. If there is no operation_id, validate the input record. Insert a new record. If there is an operation id, update the operation record. Update the op_info with the operation id. If successful return Ok(()), else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>add_or_update_operation_detail(pool: &amp;Pool&lt;ConnectionManager&gt;, operation_detail: &amp;mut OperationDetail) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: inserts or updates the operation details record. If a successful insert, the provided input operation_detail is modified. Errors if insert fails<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>operation_detail – the operation details info</p>
<p>RETURNS: Ok(()) if success, else Error</p>
<p>IMPLEMENTATION: get a single connection to the database. If there is no operation detail id, insert a new detail record. If there is an operation detail id, update the existing record. Update the operation_detail with the operation_detail_id. If successful return Ok(()), else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>save_state(pool: &amp;Pool&lt;ConnectionManager&gt;, device_detail: &amp;BlockDevice, state: State) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: save the state machine information for the device in the database<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>device_detail – the block device info</p>
<p>state – the state of the state machine</p>
<p>RETURNS: Ok(()) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Check if the device is in the database (which it should be). Update the state, start a transaction that rolls back if necessary to update the database. If successful, return Ok(()), else error out.</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>save_smart_result(pool: &amp;Pool&lt;ConnectionManager&gt;, device_detail: &amp;BlockDevice, smart_passed: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: save the result of the smart check of the device in the database<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>device_detail – the block device info</p>
<p>smart_passed – whether the smart check passed or not</p>
<p>RETURNS: Ok(()) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Check if the device is in the database(which it should be). Update smart_passed. start a transaction that rolls back if necessary to update the database. If successful, return Ok(()), else error out.</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>get_devices_from_db(pool: &amp;Pool&lt;ConnectionManager&gt;, storage_detail_id: u32) -&gt; BynarResult&lt;Vec&lt;u32, String, Pathbuf&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get the currently known disks from the database<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>storage_detail_id – the entry number of the storage detail table</p>
<p>RETURNS: Ok(device id, device name, device path) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the device id, name and path. If successful, return Ok(dev_id, dev_name, dev_path), else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>get_state(pool: &amp;Pool&lt;ConnectionManager&gt;, device_detail: u32) -&gt; BynarResult&lt;State&gt;</p>
<blockquote>
<p>DESCRIPTION: get the state information from the database<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>device_detail – the entry number of the device in the hardware table</p>
<p>RETURNS: Ok(state) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the state of the device. If successful, return Ok(state), else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>get_smart_result(pool: &amp;Pool&lt;ConnectionManager&gt;, device_detail: u32) -&gt; BynarResult&lt;bool&gt;</p>
<blockquote>
<p>DESCRIPTION: get the currently known disks from the database<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>device_detail – the entry number of the device in the hardware table</p>
<p>RETURNS: Ok(bool) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for whether the device passed the smart checks or not. If successful, return Ok(passed?), else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>row_to_ticket(row: &amp;Row&lt;’_&gt;) -&gt; DiskRepairTicket</p>
<blockquote>
<p>DESCRIPTION: convert a row from a query to a DiskRepairTicket<br />
PARAMETERS: row – the query result to convert</p>
<p>RETURNS: DiskRepairTicket<br />
IMPLEMENTATION: Create a DiskRepairTicket with the values from the row</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>get_outstanding_repair_tickets(pool: &amp;Pool&lt;ConnectionManager&gt;, storage_detail_id: u32) -&gt; BynarResult&lt;Vec&lt;DiskRepairTicket&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get a list of ticket IDs (JIRA/other ids) that belong to “me” that are pending, in progress, or op_type=WaitForReplacement<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>storage_detail_id – the entry number of the storage detail in the tables</p>
<p>RETURNS: Ok(list of disk repair tickets) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for a list of Operations that are InProgress, Pending, WaitingForReplacement, and Good with the specified storage_detail_id. Convert the rows returned into DiskRepairTickets and, if sucessful, return Ok(List of disk repair tickets), else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>resolve_ticket_in_db(pool: &amp;Pool&lt;ConnectionManager&gt;, ticket_id: &amp;str) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: set the status as Complete for the record with the given ticket_id. Note: this is equivalent to calling the add_or_update_operation_detaiL() with the appropriate fields set<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>ticket_id – the ticket id in the support ticket system</p>
<p>RETURNS: Ok(()) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Update the operation_details as OperationStatus::Complete where the ticket_id matches. If successful, return Ok(()), else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>is_hardware_waiting_repair(pool: &amp;Pool&lt;ConnectionManager&gt;, storage_detail_id: u32, device_name: &amp;str, serial_number: Option&lt;&amp;str&gt;) -&gt; BynarResult&lt;bool&gt;</p>
<blockquote>
<p>DESCRIPTION: check if the hardware/device is currently waiting for repair<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>storage_detail_id – the entry number of the storage detail</p>
<p>device_name – the name of the device to check</p>
<p>serial_number – the serial number of the device to check</p>
<p>RETURNS: Ok(bool) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the device’s Operation/Storage details. check if the OperationType is WaitingForReplacement. If successful, return Ok(true) if the device is waiting for repair, Ok(false) if the device is not waiting for repairs, or error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>get_region_id(pool: &amp;Pool&lt;ConnectionManager, region_name: &amp;str) -&gt; BynarResult&lt;Option&lt;u32&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get the region id based on the region name<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>region_name – the name of the region to get the database id value of</p>
<p>RETURNS: Ok(id number if exists) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the region name. If successful, return Ok(Some(region_id)) if the region name is in the database, Ok(None) if it is not in the database, else error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>get_storage_id(pool: &amp;Pool&lt;ConnectionManager&gt;, storage_type: &amp;str) -&gt; BynarResult&lt;Option&lt;u32&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get the storage id based on the storage type<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>storage_type – the storage type to get the database id value of</p>
<p>RETURNS: Ok(id number if exists) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the storage type. If successful return Ok(Some(storage_id)) if the storage type is in the database, Ok(None) if it is not in the database, else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>get_storage_detail_id(pool: &amp;Pool&lt;ConnectionManager&gt;, storage_id: u32, region_id: u32, host_name: &amp;str) -&gt; BynarResult&lt;Option&lt;u32&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get the storage detail id based on the storage id, region id and hostname<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>storage_id – the id of the storage type information</p>
<p>region_id – the id of the region name</p>
<p>host_name - the host name</p>
<p>RETURNS: Ok(storage detail id if exist) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the storage_detail_id associated with the input values. If successful, return Ok(Some(storage_detail_id)), Ok(None) if it does not exist, or error out</p>
</blockquote></td>
</tr>
<tr class="odd">
<td><p>get_all_pending_tickets(pool: &amp;Pool&lt;ConnectionManager&gt;) -&gt; BynarResult&lt;Vec&lt;DiskPendingTicket&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get a list of ticket IDs (JIRA/other) that belong to ALL servers that are in pending state and outstanding tickets<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>RETURNS: Ok(list of pending/outstanding disks) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for ALL tickets with the WaitingForReplacement, Pending, InProgress, and GoodState, convert them to DiskPendingTickets. If successful, return Ok(list of diskpending tickets) else error out</p>
</blockquote></td>
</tr>
<tr class="even">
<td><p>get_host_name(pool: &amp;Pool&lt;ConnectionManager&gt;, device_id: i32) -&gt; BynarResult&lt;Option&lt;String&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: get the host name based on the device id<br />
PARAMETERS: pool – the pool of connections to the database</p>
<p>device_id – the id number of the device in the database</p>
<p>RETURNS: Ok(hostname if it exists) on success, else Error<br />
IMPLEMENTATION: get a single connection to the database. Query the database for the host name associated with the device id. If successful, return Ok(Some(host_name)) or Ok(None) if the host name does not exist for the device id. Otherwise, error out</p>
</blockquote></td>
</tr>
</tbody>
</table>

# Helper Functions

## Introduction

There are a couple of functions and types that are needed across most of
the Bynar program. These include the Error Type, host information, and
various connection and requests.

## Error Module

The error module provides the error type for the Bynar program. Various
error types are imported and generalized as a BynarResult Error

### Type

#### BynarResult\<T\>

This is the generic Bynar Errortype, a Result type of type \<T,
BynarError\>

### Enums

#### PwdBError

##### Enum Values

| Name               | Description                   |
| ------------------ | ----------------------------- |
| PwdError(PwdError) | An error from the pwd library |

##### Trait Implementations

###### Display

| Name | Inputs                  | Description                             | Outputs     |
| ---- | ----------------------- | --------------------------------------- | ----------- |
| fmt  | f: \&mut fmt::Formatter | Given a PwBError, display the error msg | fmt::Result |

###### Debug

#### BynarError

##### Enum Values

| Name                             | Description                       |
| -------------------------------- | --------------------------------- |
| BlkidError(BlkidError)           | A blkid command error             |
| BlockUtilsError(BlockUtilsError) | A block\_utils library error      |
| Error(String)                    | A generic String error            |
| GojiError(GojiError)             | A Gojira library error            |
| HardwareError(HardwareError)     | A hardware error                  |
| IoError(IOError)                 | A std::io error                   |
| LvmError(LvmError)               | An lvm error                      |
| NixError(NixError)               | A nix library error               |
| ParseIntError(ParseIntError)     | A parseint error (integer parser) |
| PostgresError(PostgresError)     | A postgres command error          |
| ProtobufError(ProtobufError)     | A protobuf serializer error       |
| PwdError(PwdBError)              | A pwd error                       |
| R2d2Error(R2d2Error)             | An R2d2 error                     |
| RadosError(RadosError)           | A Ceph rados error                |
| ReqwestError(ReqwestError)       | A reqwest library error           |
| SerdeJsonError(SerdeJsonError)   | A serde json library error        |
| SlackError(SlackError)           | A Slack error                     |
| UuidError(UuidError)             | A uuid error                      |
| VaultError(VaultError)           | A vault error                     |
| ZmqError(ZmqError)               | A zmq library error               |

##### Implementation

| Name       | Inputs      | Description                                       | Outputs    |
| ---------- | ----------- | ------------------------------------------------- | ---------- |
| new        | err: String | Create a new BynarError with a String message     | BynarError |
| to\_string | self        | Convert a BynarError into a String representation | String     |

##### Trait Implementations

###### Display

| Name | Inputs                  | Description                          | Outputs     |
| ---- | ----------------------- | ------------------------------------ | ----------- |
| fmt  | f: \&mut fmt::Formatter | Given a Bynar, display the error msg | fmt::Result |

###### From\<PwdError\>

| Name | Inputs        | Description                           | Outputs    |
| ---- | ------------- | ------------------------------------- | ---------- |
| from | err: PwdError | Given a PwdError, create a BynarError | BynarError |

###### From\<String\>

| Name | Inputs      | Description                         | Outputs    |
| ---- | ----------- | ----------------------------------- | ---------- |
| from | err: String | Given a String, create a BynarError | BynarError |

###### From\<’a str\>

| Name | Inputs     | Description                        | Outputs    |
| ---- | ---------- | ---------------------------------- | ---------- |
| from | err: \&str | Given a \&str, create a BynarError | BynarError |

###### Debug, de::Error

### Structs

#### HardwareError

##### Attributes

| Name             | Type             | Description                                 |
| ---------------- | ---------------- | ------------------------------------------- |
| error            | String           | The error                                   |
| name             | String           | The name of the error                       |
| location         | Option\<String\> | The location? Of the error                  |
| location\_format | Option\<String\> | Uh, the format??????                        |
| serial\_number   | Option\<String\> | Serial number of whatever is having issues? |

##### Trait Implementations

###### Display

| Name | Inputs                  | Description                                  | Outputs     |
| ---- | ----------------------- | -------------------------------------------- | ----------- |
| fmt  | f: \&mut fmt::Formatter | Given a HardwareError, display the error msg | fmt::Result |

###### Debug

## Host Information

Gather information about the current host. Please note that Host
Information makes a few assumptions, such as that there IS a default
interface (it is possible, but VERY UNLIKELY, that there is no default
interface). Once a gateway is found, the FIRST ip address that is an
ipv4 address is returned. That means, that if there are multiple valid
IPv4 addresses, only the first seen will be used.

### Enums

#### StorageTypeEnum

The type of distributed storage

##### Enum Values

| Name    | Description          |
| ------- | -------------------- |
| Ceph    | Ceph storage type    |
| Scaleio | Scaleio storage type |
| Gluster | Gluster storage type |
| Hitachi | Hitachi storage type |

##### Trait Implementations

###### Display

| Name | Inputs                  | Description                                       | Outputs     |
| ---- | ----------------------- | ------------------------------------------------- | ----------- |
| fmt  | f: \&mut fmt::Formatter | Given a StorageTypeEnum, display the storage type | fmt::Result |

###### Debug

### Structs

#### Host

##### Attributes

| Name                  | Type                          | Description              |
| --------------------- | ----------------------------- | ------------------------ |
| hostname              | String                        | The host name            |
| ip                    | IpAddr                        | The ip address           |
| region                | String                        | The region               |
| kernel                | String                        | The kernel type          |
| server\_type          | String                        | The server type          |
| serial\_number        | String                        | The serial number        |
| machine\_architecture | String                        | The machine architecture |
| scsi\_info            | Vec\<block\_utils::ScsiInfo\> | The scsi information     |
| storage\_type         | StorageTypeEnum               | The storage type         |
| array\_name           | Option\<String\>              | The array name           |
| pool\_name            | Option\<String\>              | The pool name            |

##### Implementation

| Name | Inputs | Description       | Outputs             |
| ---- | ------ | ----------------- | ------------------- |
| new  | N/A    | Create a new Host | BynarResult\<Host\> |

##### Trait Implementations

###### Debug

### Helper Functions

<table>
<thead>
<tr class="header">
<th>Helper Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>get_default_iface() -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: get the default interface</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(default interface) on success, Error otherwise</p>
<p>IMPLEMENTATION: open the /proc/net/route file. For each line, try to find the default gateway “00000000” and return the interface. If successfule, return Ok(default interface) else error</p></td>
</tr>
<tr class="even">
<td><p>get_ip() -&gt; BynarResult&lt;IpAddr&gt;</p>
<p>DESCRIPTION: Find the IP address on the default interface</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(ip address) on success, Error otherwise</p>
<p>IMPLEMENTATION: get all interfaces as well as the default interface. filter all interfaces to get the default. In the interface, loop through the ip addresses until an ipv4 address is found and return it. If successful, return the ipv4 address, else error.</p></td>
</tr>
<tr class="odd">
<td><p>get_region_from_hostname(hostname: &amp;str) -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: Get the region from the hostname</p>
<p>PARAMETERS: hostname – the hostname</p>
<p>RETURNS: Ok(region) on success, Error otherwise</p>
<p>IMPLEMENTATION: Production hostnames are usually in the format name-regionpart1-regionpart2-*, so split the hostname by ‘-’, skip the first sub string and combine the region parts. If successful, either return Ok(region) if totally successful, Ok(“test-region”) if the hostname is not regular, or error if it fails.</p></td>
</tr>
<tr class="even">
<td><p>get_storage_type() -&gt; BynarResult&lt;StorageTypeEnum&gt;</p>
<p>DESCRIPTION: get the storage type used on this system</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(storage type) on success, Error otherwise</p>
<p>IMPLEMENTATION: for now, it just returns Ceph....</p></td>
</tr>
<tr class="odd">
<td><p>server_type() -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: Find the server type</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(server type) on success, Error otherwise</p>
<p>IMPLEMENTATION: Go to /sys/class/dmi/id/product_name and read the file. If successful return the file contents as Ok(server type), else error</p></td>
</tr>
<tr class="even">
<td><p>server_serial() -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: get the server serial number</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(server serial number) on success, Error otherwise</p>
<p>IMPLEMENTATION: for now, it just tries the easy way, which is reading the /sys/class/dmi/id/product_serial file for the number. If successful returns Ok(server serial number), otherwise error</p></td>
</tr>
</tbody>
</table>

## Helper Module

Public functions and structures that can be used outside of the library.

### Structs

#### ConfigSettings

##### Attributes

| Name                  | Type                          | Description              |
| --------------------- | ----------------------------- | ------------------------ |
| hostname              | String                        | The host name            |
| ip                    | IpAddr                        | The ip address           |
| region                | String                        | The region               |
| kernel                | String                        | The kernel type          |
| server\_type          | String                        | The server type          |
| serial\_number        | String                        | The serial number        |
| machine\_architecture | String                        | The machine architecture |
| scsi\_info            | Vec\<block\_utils::ScsiInfo\> | The scsi information     |
| storage\_type         | StorageTypeEnum               | The storage type         |
| array\_name           | Option\<String\>              | The array name           |
| pool\_name            | Option\<String\>              | The pool name            |

##### Trait Implementations

###### Clone, Debug, Deserialize

#### DBConfig

##### Attributes

| Name     | Type             | Description                      |
| -------- | ---------------- | -------------------------------- |
| username | String           | Database username                |
| password | Option\<String\> | Database password                |
| port     | u16              | Port to connect to database with |
| endpoint | String           | Database endpoint                |
| dbname   | String           | Database name                    |

##### Trait Implementations

###### Clone, Debug, Deserialize

### Helper Functions

<table>
<thead>
<tr class="header">
<th>Helper Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>load_config&lt;T&gt;(config_dir: &amp;Path, name: &amp;str) -&gt; BynarResult&lt;T&gt;</p>
<p>DESCRIPTION: load a config file that is deserializable</p>
<p>PARAMETERS: config_dir – the directory of the config file</p>
<blockquote>
<p>name – name of the file to deserialize</p>
</blockquote>
<p>RETURNS: Ok(deserialized structure) on success, Error otherwise</p>
<p>IMPLEMENTATION: create the path to the file, and check if it exists. Read the file and deserialize it into the struct. If successfule, return Ok(deserialized struct) otherwise error out</p></td>
</tr>
<tr class="even">
<td><p>connect(host: &amp;str, port: &amp;str, server_publickey: &amp;str) -&gt; BynarResult&lt;Socket&gt;</p>
<p>DESCRIPTION: connect to the input host:port ip and securing with the server public key</p>
<p>PARAMETERS: host – the host ip address</p>
<blockquote>
<p>port – the port to connect to</p>
<p>server_publickey – the public key of the server used to secure the socket</p>
</blockquote>
<p>RETURNS: Ok(connected socket) on success, Error otherwise</p>
<p>IMPLEMENTATION: create a new zmq REQ socket. create a curveKeyPair to secure the socket. set the keys in the socket and connect using tcp to the host:port ip address. If successful, return Ok(REQ socket), otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>get_vault_token(endpoint: &amp;str, token: &amp;str, hostname: &amp;str) -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: get the vault secret from the Hashicorp Vault</p>
<p>PARAMETERS: endpoint – the hashicorp endpoint</p>
<blockquote>
<p>token – token to access the vault with</p>
<p>hostname – name of the host to get the secret of</p>
</blockquote>
<p>RETURNS: Ok(vault secret??) on success, Error otherwise</p>
<p>IMPLEMENTATION: Connect to the Vault with VaultClient, and get the secret. If successful return Ok(Vault secret) else error</p></td>
</tr>
<tr class="even">
<td><p>add_disk_request(s: &amp;mut Socket, path: &amp;Path, id: Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: send a request to add a disk to a cluster</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<blockquote>
<p>path – the path of the disk to add to the cluster</p>
<p>id – the osd id of the disk to add</p>
<p>simulate – if passed, skip evaluation</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the Operation message. Convert the message into bytes and send it from the socket and wait for a response. Parse the Operation result for OK or ERROR. If successful, return Ok(()), otherwise something failed.</p></td>
</tr>
<tr class="odd">
<td><p>list_disks_request(s: &amp;mut Socket) -&gt; BynarResult&lt;Vec&lt;Disk&gt;&gt;</p>
<p>DESCRIPTION: send a request to get a list of disks from a cluster</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<p>RETURNS: Ok(disk list) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the Operation message. Convert the message into bytes and send it from the socket and wait for a response. Parse the Operation result for the list of disks. If successful, return Ok(disk list), otherwise something failed.</p></td>
</tr>
<tr class="even">
<td><p>safe_to_remove_request(s: &amp;mut Socket, path: &amp;Path) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: send a request to a cluster to ask if a disk is safe to remove</p>
<p>PARAMETERS: s – the socket to send messages from</p>
<blockquote>
<p>path – the path of the disk to check if removable</p>
</blockquote>
<p>RETURNS: Ok(is safe to remove?) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the Operation message. Convert the message into bytes and send it from the socket and wait for a response. Parse the Operation result for whether the disk is safe to remove. If successful, return Ok(true) if safe to remove, Ok(false) if the disk is not safe to remove, otherwise something failed so error out.</p></td>
</tr>
<tr class="odd">
<td><p>remove_disk_request(s: &amp;mut Socket, path: &amp;Path, id: Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: send a request to remove a disk from a cluster</p>
<p>PARAMETERS: s – the socket to send messages from</p>
<blockquote>
<p>path – the path of the disk to add to the cluster</p>
<p>id – the osd id of the disk to add</p>
<p>simulate – if passed, skip evaluation</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create the Operation message. Convert the message into bytes and send it from the socket and wait for a response. Parse the Operation result for OK or ERROR. If successful, return Ok(()), otherwise something failed.</p></td>
</tr>
<tr class="even">
<td><p>get_jira_tickets(s: &amp;mut Socket) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: send a request to get Jira tickets</p>
<p>PARAMETERS: s – the socket to send messages from</p>
<blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Create the Operation message. Convert the message into bytes and send it from the socket and wait for a response. Parse the Operation result for OK or ERROR. If Ok get the tickets and print the ticket information. If successful, return Ok(()), otherwise something failed.</p></td>
</tr>
</tbody>
</table>

# Client 

## Introduction

This is a client interface built as a separate binary. It enables a user
to make manual calls to the disk\_manager and Bynar.

### Client Interface

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>add_disk(s: &amp;mut Socket, path: &amp;Path, id: Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Send a message to add a disk to the cluster</p>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>path – the path of the disk to add</p>
<p>id – the optional osd id of the disk to add</p>
<p>simulate – if passed, skip evaluation of the function</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the Helper library add_disk_request function. If successful return Ok(()), else error</p></td>
</tr>
<tr class="even">
<td><p>list_disks(s: &amp;mut Socket) -&gt; BynarResult&lt;Vec&lt;Disk&gt;&gt;</p>
<blockquote>
<p>DESCRIPTION: list the disks in a cluster and print them to the console</p>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Run the helper library list_disks_request and print the disks. If successful return Ok(()), else error</p></td>
</tr>
<tr class="odd">
<td><p>remove_disk(s: &amp;mut Socket, path: &amp;Path, id: Option&lt;u64&gt;, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Send a message to remove a disk from the cluster</p>
</blockquote>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>path – the path of the disk to add</p>
<p>id – the optional osd id of the disk to add</p>
<p>simulate – if passed, skip evaluation of the function</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Run the helper library remove_disk_request. If successful return Ok(()), else error</p></td>
</tr>
<tr class="even">
<td><p>handle_add_disk(s: &amp;mut Socket, matches: &amp;ArgMatches&lt;’_&gt;)</p>
<blockquote>
<p>DESCRIPTION: Wrapper for adding a disk, parses a command line input to add a disk</p>
</blockquote>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>matches – the argument inputs parsed from the command line</p>
<p>RETURNS: None</p>
</blockquote>
<p>IMPLEMENTATION: get the arguments from the match input, and check their types. Run the add_disk function on the inputs. If successful print a success message to the terminal, else print the failure message</p></td>
</tr>
<tr class="odd">
<td><p>handle_list_disks(s: &amp;mut Socket)</p>
<blockquote>
<p>DESCRIPTION: Wrapper for listing disks</p>
</blockquote>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>RETURNS: None</p>
</blockquote>
<p>IMPLEMENTATION: list the disks using the list_disks function and print the list if successful, otherwise print the error message</p></td>
</tr>
<tr class="even">
<td><p>handle_jira_tickets(s: &amp;mut Socket) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Wrapper for getting and printing jira tickets</p>
</blockquote>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: use the helper library get_jira_tickets function. If successful, return Ok(()), otherwise error out</p></td>
</tr>
<tr class="odd">
<td><p>handle_remove_disk(s: &amp;mut Socket, matches: &amp;ArgMatches&lt;’_&gt;)</p>
<blockquote>
<p>DESCRIPTION: Wrapper for removing a disk, parses a command line input to remove a disk</p>
</blockquote>
<p>PARAMETERS: s – the socket to send and receive messages</p>
<blockquote>
<p>matches – the argument inputs parsed from the command line</p>
<p>RETURNS: None</p>
</blockquote>
<p>IMPLEMENTATION: get the arguments from the match input, and check their types. Run the remove_disk function on the inputs. If successful print a success message to the terminal, else print the failure message</p></td>
</tr>
<tr class="even">
<td><p>get_cli_args(default_server_key: &amp;str) -&gt; ArgMatches&lt;’_&gt;</p>
<blockquote>
<p>DESCRIPTION: Create the command line arguments and parse them for proper input</p>
</blockquote>
<p>PARAMETERS: default_server_key – the default value for the server key</p>
<blockquote>
<p>RETURNS: An ArgMatches with the matched arguments to the cli inputs</p>
</blockquote>
<p>IMPLEMENTATION: Create the App Ceph Disk Manager Client and add the RPC calls. Calls include host, port, server_key, with subcommands add, list, get_jira_tickets, remove, and v for verbosity. Run get_matches on the App object to get the command line arguments matching the CLI created in App. return the matches.</p></td>
</tr>
<tr class="odd">
<td><p>main()</p>
<blockquote>
<p>DESCRIPTION: Run the Client</p>
</blockquote>
<p>PARAMETERS: None</p>
<blockquote>
<p>RETURNS: None</p>
</blockquote>
<p>IMPLEMENTATION: create the server key. Get the CLI arguments. match the –v flags to level of verbosity. Get the host and port values for creating sockets. get the server publick key, and use the helper library to connect to the server. depending on the subcommand, either run handle_add_disk, handle_list_disks, handle_remove_disk, or handle_jira_tickets.</p></td>
</tr>
</tbody>
</table>

# Support Tickets

## Introduction

Bynar won’t always be able to handle a disk problem. So, if for whatever
reason Bynar cannot fix a disk or remove it immediately, it needs to be
able to create a support ticket. Bynar also needs to be able to scan
opened tickets to see if they’ve been resolved, so that Bynar can add
the fixed disks back in. For now, the only ticket system supported is
JIRA.

### JIRA Support

JIRA is a support ticketing system. We need to be able to create tickets
and scan and list them as well.

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>create_support_ticket(settings: &amp;ConfigSettings, title: &amp;str, description:&amp;str) -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: Create a new JIRA support ticket and return the ticket ID associated with it</p>
<p>PARAMETERS: settings – the configuration settings containing the information necessary to log into JIRA and use the API</p>
<blockquote>
<p>title – the title of the new ticket</p>
<p>description – the description of the new ticket</p>
</blockquote>
<p>RETURNS: Ok(ticket ID) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create an Issue object, filling in the Assignee, component, description, priority, project, and summary attributes. Most of the these are given from the Config Settings. Open the proxy if there is one, and create a reqwest Client with a proxy. Create a Jira object (connect to Jira) and create a new Issue with the description in Jira. If successful, return Ok(created ticket ID), otherwise error out.</p></td>
</tr>
<tr class="even">
<td><p>ticket_resolved(settings: &amp;ConfigSettings, issue_id: &amp;str) -&gt; BynarResult&lt;bool&gt;</p>
<blockquote>
<p>DESCRIPTION: check to see if a JIRA support ticket is marked as resolved</p>
<p>PARAMETERS: settings – config settings needed to connect to JIRA</p>
<p>issue_id – the ID of the ticket to check</p>
<p>RETURNS: Ok(bool) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Connect to JIRA (with or without a proxy). Open the issue and check if the ticket is resolved. If successful, return Ok(true) if the issue is resolved, Ok(false) if the ticket is not yet resolved, else error out.</p></td>
</tr>
</tbody>
</table>

# Disk Manager

## Introduction

This program handles the adding and removing of disks from a server

## Disk Manager

### Structs

#### DiskManagerConfig

##### Attributes

| Name            | Type             | Description               |
| --------------- | ---------------- | ------------------------- |
| backend         | BackendType      | The backend of the server |
| vault\_token    | Option\<String\> | Hashicorp vault token     |
| vault\_endpoint | Option\<String\> | Hashicorp vault endpoint  |

##### Trait Implementations

###### Clone, Debug, Deserialize

### Functions

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>convert_media_to_disk_type(m: &amp;MediaType) -&gt; DiskType</p>
<p>DESCRIPTION: convert a MediaType object into a DiskType object</p>
<p>PARAMETERS: m – the object to convert</p>
<p>RETURNS: converted DiskType object</p>
<p>IMPLEMENTATION: convert the MediaType to a DiskType and return it</p></td>
</tr>
<tr class="even">
<td><p>setup_curve(s: &amp;mut Socket, config_dir: &amp;Path, vault: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: Set up a curve encryption scheme on a socket</p>
<p>PARAMETERS: s – socket to set the curve encryption on</p>
<p>config_dir – the config file directory</p>
<p>vault – whether using Hashicorp vault to set the encryption</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: set the socket with a curve server. Create a new CurveKeyPair. Get the hostname and get the key file from the config directory. If using the Hashicorp vault, connect to the vault and set a new secret with the generated keypair and set the socket with the keypair. Otherwise, if not using vault, just set the socket with the secret key and save the key to a file. If successful, return Ok(()), otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>listen(backend_type: BackendType, config_dir: &amp;Path, listen_address: &amp;str, vault: bool) -&gt; BynarResult&lt;()&gt;</p>
<blockquote>
<p>DESCRIPTION: listen for Operation messages from the listen address and run any successfully received messages.</p>
<p>PARAMETERS: backend_type – the backend type of the server</p>
<p>config_dir – the config file directory</p>
<p>listen_address – the address of the client to listen to</p>
<p>vault – whether the program is using the hashicorp vault or not</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
</blockquote>
<p>IMPLEMENTATION: Create a Responder Socket and set up the curve encryption on the socket. Bind the socket (listen) to the listen_address using tcp. Loop, and while looping wait to for a message (in bytes). Parse an Operation message from the bytes and check the Op type. If an Add operation, check if it has the necessary fields and run add_disk. If AddPartition, do nothing (for now). If List, run list_disks. If Remove, check if the message has the necessary fields and run remove_disk. If SafeToRemove, check if the message has the necessary fields and run safe_to_remove_disk. If GetCreatedTickets, run get_jira_tickets. sleep for 10 milliseconds between each operation. If successful, it should loop continuously until the program is stopped (in which case return Ok(())), otherwise it should error out.</p></td>
</tr>
<tr class="even">
<td><p>respond_to_client&lt;T: protobuf::Message&gt;(result: &amp;T, s: &amp;mut Socket) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: send a response back to the client with the result of an operation</p>
<p>PARAMETERS: result – the result of an operation</p>
<blockquote>
<p>s – the socket to send and receive messages from</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: convert the message to bytes and send the bytes to the client. If successful, return Ok(()), else error out.</p></td>
</tr>
<tr class="odd">
<td><p>add_disk(s: &amp;mut Socket, d: &amp;str, backend: &amp;BackendType, id: Option&lt;u64&gt;, config_dir: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: try to add a disk to the server and send the result back to the requestor</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<blockquote>
<p>d– the disk device path to add</p>
<p>backend – the backend type</p>
<p>id – the osd id to use</p>
<p>config_dir – the configuration file directory</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Load the backend (For now only Ceph). Run backend’s add_disk function and check the result. Set the OpResult’s attributes depending on the result of the add_disk, and send the OpResult to the client. If successful, return Ok(()), else error out.</p></td>
</tr>
<tr class="even">
<td><p>get_disks() -&gt; BynarResult&lt;Vec&lt;Disk&gt;&gt;</p>
<p>DESCRIPTION: try to get a list of Disks from the server</p>
<p>PARAMETERS: None</p>
<p>RETURNS: Ok(list of Disks) on success, Error otherwise</p>
<p>IMPLEMENTATION: Search for all block devices. Gather the udev info of all found block devices. For each device, create a new Disk object, get its partition info (blank disks will fail), translate the block_utils mediatype to the DiskType (from Protobuf), set the various values in the Disk, and add it to the list of Disks. If successful, return Ok(list of disks), otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>get_partition_info(dev_path: &amp;Path) -&gt; BynarResult&lt;PartitionInfo&gt;</p>
<p>DESCRIPTION: get partition info of a device/disk</p>
<p>PARAMETERS: dev_path – the device/disk path</p>
<p>RETURNS: Ok(partition info) on success, Error otherwise</p>
<p>IMPLEMENTATION: create a new Partition Info. Read the header of the disk, then read the partitions using the header. Transform the returned partitions into protobuf PartitionInfo. If successful, return Ok(partition info), else error out.</p></td>
</tr>
<tr class="even">
<td><p>list_disks(s: &amp;mut Socket) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: get a list of disks on the server and send it to the client</p>
<p>PARAMETERS: s – the socket to send and receive from</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the list of disks with get_disks. Create the Disks message and set the disks. Write the Disks message to bytes and send to the client.</p></td>
</tr>
<tr class="odd">
<td><p>remove_disk(s: &amp;mut Socket, d: &amp;str, backend: &amp;BackendType, config_dir: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: try to remove a disk from the server and send the result back to the client</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<blockquote>
<p>d– the disk device path to remove</p>
<p>backend – the backend type</p>
<p>config_dir – the configuration file directory</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Load the backend (For now only Ceph). Run backend’s remove_disk function and check the result. Set the OpResult’s attributes depending on the result of the remove_disk, and send the OpResult to the client. If successful, return Ok(()), else error out.</p></td>
</tr>
<tr class="even">
<td><p>safe_to_remove(d: &amp;Path, backend: &amp;BackendType, config_dir: &amp;Path) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: check if a disk is safe to remove</p>
<p>PARAMETERS: d– the disk device path to check if safe to remove</p>
<blockquote>
<p>backend – the backend type</p>
<p>config_dir – the configuration file directory</p>
</blockquote>
<p>RETURNS: Ok(bool) on success, Error otherwise</p>
<p>IMPLEMENTATION: load the backend, and run the backend safe_to_remove function. If successful, return Ok(true) if safe to remove, Ok(false) if not safe to remove, or error out.</p></td>
</tr>
<tr class="odd">
<td><p>safe_to_remove_disk(s: &amp;mut Socket, d: &amp;str, backend: &amp;BackendType, config_dir: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Check if a disk is safe to remove and send the result to the client</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<blockquote>
<p>d– the disk device path to check if safe to remove</p>
<p>backend – the backend type</p>
<p>config_dir – the configuration file directory</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: create the OpBoolResult message. Run the safe_to_remove function. Based on the output fill out the OpBoolResult message and convert it to bytes and send to the client. If successful, return Ok(()), otherwise error out.</p></td>
</tr>
<tr class="even">
<td><p>get_jira_tickets(s: &amp;mut Socket, config_dir: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: get a list of JIRA tickets and send the list to the client</p>
<p>PARAMETERS: s – the socket to send and receive messages from</p>
<blockquote>
<p>config_dir – the configuration file directory</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: create an OpJiraTicketsResult Message. Load the config settings and connect to the database. get all pending tickets from the database, and set the tickets in the OpJiraTicketsMessage, and send the message to the client. If successful, return Ok(()), otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>main()</p>
<p>DESCRIPTION: run the disk manager</p>
<p>PARAMETERS: None</p>
<p>RETURNS: None</p>
<p>IMPLEMENTATION: Create the Command Line Interface and parse the arguments passed in. Check the verbosity and set the logger, and check other CLI inputs. Then, run listen.</p></td>
</tr>
</tbody>
</table>

# Disk Testing

## Introduction

This is the disk testing mechanism of Bynar, which uses a State Machine
to check the health of a disk and determine whether it has failed or
not, as well as whether it needs replacement or intervention. Disk
checks are defined and tested, using the state machine to determine what
is and is not possible. The state machine itself can be output as a
visual diagram when one of the unit tests is run.

## State Machine

The state machine is set up by adding all the transition states into
itself, with each state ordered from the most to least ideal outcome.

The state machine, when run, will attempt to run all transitions until
an end state is reached and return. It will start from the current state
that the machine is in, and loop through all possible next states
(edges). If a transition returns Fail, try the next path until all paths
are exhausted.

### Special Cases

Depending on the Storage Type, special cases might arise. Some
transitions may not be possible.

#### Ceph

Ceph Journals are a special type of device/partition that act akin to
cache. A part of this special property includes that a ceph journal
can’t be mounted. This includes the disk the partitions are on.
However, a smartmon/smartctl scan can still be run on the disk as a
health check. However, since the disk cannot be mounted, nor would it
have a filesystem, no filesystem, write, read, or wear checks can be
made on these types of disks.

### Type

#### TransitionFn

A function type fn(State, \&mut BlockDevice, \&Option\<ScsiInfo,
Option\<ScsiInfo\>)\> bool) -\> State

The Transition function defines a transition between two states, given
some information on the current Block Device and information gathered
from the scsi commands.

### Trait

#### Transition

Transition trait that defines a transition between two states, given
some Event, and uses a database connection to save and resume a state.
The input state is the state to transition to if the Event is
successful.

##### Trait Function Definition

<table>
<thead>
<tr class="header">
<th>Name</th>
<th>Inputs</th>
<th>Description</th>
<th>Outputs</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>transition</td>
<td><p>to_state: State</p>
<p>device: &amp;mut BlockDevice</p>
<p>scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;</p>
<p>simulate: bool</p></td>
<td>Transition from the current state to an ending state given an Event.</td>
<td>State</td>
</tr>
</tbody>
</table>

### Enums

#### State

A State in the state machine

##### Enum Values

| Name                  | Description                                                              |
| --------------------- | ------------------------------------------------------------------------ |
| Corrupt               | The disk or disk filesystem is corrupted. Repairs are attempted          |
| Fail                  | The Transition failed (for whatever reason)                              |
| Good                  | The filesystem is good                                                   |
| Mounted               | The disk was able to be mounted                                          |
| MountFailed           | Mounting the disk failed                                                 |
| NotMounted            | The disk is not mounted                                                  |
| ReadOnly              | The device is mounted as read only                                       |
| ReformatFailed        | Tried to reformat but failed                                             |
| Reformatted           | Reformatting the device succeeded                                        |
| RepairFailed          | Tried to repair corruption and failed                                    |
| Repaired              | Repair corruption succeeded                                              |
| Replaced              | Disk was successfully replaced                                           |
| Scanned               | Disk is successfully scanned                                             |
| Unscanned             | Disk has not been scanned? Scanning failed?                              |
| WaitingForReplacement | The disk could not be repaired and needs to be replaced                  |
| WornOut               | The disk spindle is worn out and the drive will need to be replaced soon |
| WriteFailed           | Write test failed                                                        |

##### Trait Implementations

###### Display

| Name | Inputs                  | Description                                   | Outputs     |
| ---- | ----------------------- | --------------------------------------------- | ----------- |
| fmt  | f: \&mut fmt::Formatter | Given a State, display the object as a string | fmt::Result |

###### FromStr

| Name      | Inputs   | Description                    | Outputs             |
| --------- | -------- | ------------------------------ | ------------------- |
| from\_str | s: \&str | Given a string, return a state | BynarError\<State\> |

###### Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd

#### Fsck

The result of an fsck Linux command

##### Enum Values

| Name    | Description                   |
| ------- | ----------------------------- |
| Ok      | Fsck resulted in okay         |
| Corrupt | Filesystem is corrupt somehow |

### Structs

#### BlockDevice

A Block Device object, containing metadata and other information about
the device

##### Attributes

| Name                 | Type                       | Description                                         |
| -------------------- | -------------------------- | --------------------------------------------------- |
| device               | Device                     | Device information                                  |
| dev\_path            | PathBuf                    | The path to the device                              |
| device\_database\_id | Option\<u32\>              | The id of the device in the database                |
| mount\_point         | Option\<PathBuf\>          | The mount point of the device                       |
| partitions           | BTreeMap\<u32, Partition\> | A map of the partitions in the device               |
| scsi\_info           | ScsiInfo                   | Scsi Information on the device                      |
| state                | State                      | Current state of the device                         |
| storage\_detail\_id  | u32                        | The storage detail id of the device in the database |
| operation\_id        | Option\<u32\>              | The operation id of the device n the database       |

##### Implementation

| Name                      | Inputs                    | Description                                                        | Outputs |
| ------------------------- | ------------------------- | ------------------------------------------------------------------ | ------- |
| set\_device\_database\_id | device\_database\_id: u32 | set the device\_database\_id to the id of the disk in the database | None    |

##### Trait Implementations

###### Clone, Debug

#### StateMachine

A State Machine

##### Attributes

| Name          | Type                                      | Description                                                                                                               |
| ------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| dot\_graph    | Vec\<(State, State, String)\>             | A record of transitions to be written as a dot graph for visual debugging                                                 |
| graph         | GraphMap\<State, TransitionFn, Directed\> | Mapping of valid From -\> To transitions                                                                                  |
| block\_device | BlockDevice                               | The block device                                                                                                          |
| scsi\_info    | Option\<(ScsiInfo, Option\<ScsiInfo\>)\>  | Option info of this device and optional scsi host information used to determine if the device is behind a RAID controller |
| simulate      | bool                                      | Whether a simulation or not                                                                                               |

##### Implementation

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>new(block_device: BlockDevice, scsi_info: Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, simulate: bool) -&gt; StateMachine</p>
<p>DESCRIPTION: Create a new State Machine</p>
<p>PARAMETERS: block_device – the block device to create a State Machine of</p>
<blockquote>
<p>scsi_info – the optional information of the device to determine if it is RAID</p>
<p>simulate – whether running the state machine is real or simulated</p>
</blockquote>
<p>RETURNS: StateMachine</p>
<p>IMPLEMENTATION: create a new StateMachine and set the Vec and GraphMap as empty, and fill in the other attributes with their matching inputs and return the new StateMachine</p></td>
</tr>
<tr class="even">
<td><p>add_transition(&amp;mut self, from_state: State, to_state: State, callback: TransitionFn, transition_label: &amp;str)</p>
<p>DESCRIPTION: add a transition to the state machine</p>
<p>PARAMETERS: from_state – the initial state</p>
<blockquote>
<p>to_state – the state to transition to if the transition function is successful</p>
<p>callback – the transition function to attempt</p>
<p>transition_label – label used to debug the dot graph creation</p>
</blockquote>
<p>RETURNS: StateMachine with transition added</p>
<p>IMPLEMENTATION: push the from state, to_state, and transition label onto the dot graph, and add an edge to the graph from the from_state to the to_state using the callback as the transition function</p></td>
</tr>
<tr class="odd">
<td><p>run(&amp;mut self)</p>
<p>DESCRIPTION: run all transitions until no more transitions can be run and return</p>
<p>PARAMETERS: None</p>
<p>RETURNS: None</p>
<p>IMPLEMENTATION: Loop, and loop again for each edge in the current state, check the next state in the transition. If the state is Fail, continue, if the state is WaitingForReplacement break the outer loop, if the state is Good break the outer loop, otherwise continue. before looping, if the transition succeeded, save the state and loop again (does not detect unexpected values). Once the edge loop ends we should have advanced the state machine, if not there is an infinite loop, thus we break the loop, otherwise continue the process</p></td>
</tr>
<tr class="even">
<td><p>print_graph(&amp; self)</p>
<p>DESCRIPTION: print the state machine as a graph</p>
<p>PARAMETERS: None</p>
<p>RETURNS: None</p>
<p>IMPLEMENTATION: for each entry in the dot graph, walk the graph and create a Dot and add in the edges between the states</p></td>
</tr>
<tr class="odd">
<td><p>setup_state_machine(&amp;mut self)</p>
<p>DESCRIPTION: add all the transition states here.</p>
<p>PARAMETERS: None the dot graph creation</p>
<p>RETURNS: StateMachine with transition added</p>
<p>IMPLEMENTATION: Add transitions to the state graph, which will be run in they order they are added, for multiple edges the states are ordered from the most to the least ideal outcome.</p></td>
</tr>
</tbody>
</table>

##### Trait Implementations

######  Debug

| Name | Inputs                         | Description                                | Outputs     |
| ---- | ------------------------------ | ------------------------------------------ | ----------- |
| fmt  | f: \&mut fmt::Formatter\<’\_\> | Given a formatter, write a debug statement | fmt::Result |

#### AttemptRepair

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, simulate: bool) -&gt; State</p>
<p>DESCRIPTION: Given a Corrupt state, attempt to repair the filesystem on the disk</p>
<p>PARAMETERS: to_state – the end state to transition to if event successful</p>
<blockquote>
<p>device – the block device information needed to attempt a repair</p>
<p>_scsi_info – this parameter is unused</p>
<p>simulate – if passed, skip the evaluation of this function</p>
</blockquote>
<p>RETURNS: State after attempting to repair the filesystem</p>
<p>IMPLEMENTATION: if not a simulation, attempt to repair the filesystem. If successful, return the input end state, otherwise the repair failed, so return State::Fail. If a simulation, return the input end state value</p></td>
</tr>
</tbody>
</table>

###### Debug

#### CheckForCorruption

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, simulate: bool) -&gt; State</p>
<p>DESCRIPTION: Check if there is corruption on the disk and return an end state of Corrupted if so.</p>
<p>PARAMETERS: to_state – the end state to transition to if the filesystem is corrupt</p>
<blockquote>
<p>device – the block device information needed to attempt a check</p>
<p>_scsi_info – this parameter is unused</p>
<p>simulate – if passed, skip the evaluation of this function</p>
</blockquote>
<p>RETURNS: State after checking the filesystem</p>
<p>IMPLEMENTATION: if not a simulation, attempt to check if the filesystem is corrupt. If the check returns Ok, then the filesystem might have some other problem, or the filesystem could be read only, so return State::Fail. If it returns Corrupt, then return the end_state input (State::Corrupt). If it errors, then the filesystem check failed, so return State::Fail. If a simulation, return the input end state value</p></td>
</tr>
</tbody>
</table>

###### Debug

#### CheckWearLeveling

This transition currently not working properly. Checking the wear
leveling is heavily dependent on the make and model of the drive, so if
a smartctl command parser is implemented, it might not be accurate or
usable on all drives for checking the wear level as not all drives can
even check the wear level. Please note that wear level is an SSD drive

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, _device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: attempt to check if the wear level on a drive is near fail levels.</p>
<p>PARAMETERS: to_state – the end state to transition to if the drive is worn out</p>
<blockquote>
<p>_device – this parameter currently unused</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking the wear level</p>
<p>IMPLEMENTATION: Currently just returns the end state.</p>
<p>What it SHOULD do is check the wear level, and if the wear level is worn out return the end state, otherwise return a State::Fail (in the event of the check erroring out or the check returning that the drive passed all of the smart checks and the wear level is still good if the drive can even check the wear level, assuming the drive is SMART aware...)</p></td>
</tr>
</tbody>
</table>

###### Debug

#### CheckReadOnly

This transition currently not "implemented”.

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(_to_state: State, _device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: attempt to check if the device? Is read only</p>
<p>PARAMETERS: _to_state – this parameter is currently unused</p>
<blockquote>
<p>_device – this parameter currently unused</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking for read-only device</p>
<p>IMPLEMENTATION: Currently just returns the end state.</p>
<p>What it SHOULD do is check for read-only....something, and if the device or filesystem or whatever is readonly return the input end state, otherwise return State::Fail. You could parse the /proc/mounts file for “ro”, or check if the /sys/block/xxx/ro file contents is == 1</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Eval

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: attempt to check if the scanned drive is good</p>
<p>PARAMETERS: to_state – the end state to return if check passes</p>
<blockquote>
<p>device – the device information needed to evaluate the drive</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking if the device is good</p>
<p>IMPLEMENTATION: checks if the disk is blank. If so, assuming a blank disk is good, return the end_state. If not blank, check the filesystem’s LVM (if it uses an LVM) and if it does not error return the end_state. Check (if there is no mount point) if mounting the device temporarily works. Then check if the mount is writable. If the mount is writable, clean up the mount used by unmounting the device, and return the end state. If the write to mount fails, return State::WriteFailed. Otherwise error outs should return in returning State::Fails.</p></td>
</tr>
</tbody>
</table>

###### Debug

#### MarkForReplacement

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: if a drive is Worn Out, mark the drive for replacement and return the input end state</p>
<p>PARAMETERS: to_state – the end state to return marking is successful</p>
<blockquote>
<p>device – the device information needed to evaluate the drive</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking if the device is good</p>
<p>IMPLEMENTATION: Currently just returns the end state.</p>
<p>What it SHOULD do is mark the drive as needing replacement</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Mount

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: try to mount a drive, and return the input end state if successful</p>
<p>PARAMETERS: to_state – the end state to return if mounting is successful</p>
<blockquote>
<p>device – the device information needed to mount the drive</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after trying to mount a device temporarily</p>
<p>IMPLEMENTATION: Returns the input end state if mounting and unmounting is successful, otherwise return State::Fail</p></td>
</tr>
</tbody>
</table>

###### Debug

#### NoOp

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, _device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: Do nothing</p>
<p>PARAMETERS: to_state – the end state to return after doing nothing</p>
<blockquote>
<p>_device – the device information needed to evaluate the drive</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after doing nothing</p>
<p>IMPLEMENTATION: Currently just returns the end state.</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Remount

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, _device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: attempt to remount a disk if possible</p>
<p>PARAMETERS: to_state – the end state to return if remounting is successful</p>
<blockquote>
<p>_device – this parameter is currently unused</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking if the device can be remounted</p>
<p>IMPLEMENTATION: Run the remount command (mount with remount flags). If successful, return the end state input to the function. Otherwise, return State::Fail</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Replace

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, _device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: check if the drive has been replaced and the host can see it.</p>
<p>PARAMETERS: to_state – the end state to return if the disk has been successfully replaced</p>
<blockquote>
<p>device – the device information needed to check if the host can see</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after checking if the device is replaced and viewable</p>
<p>IMPLEMENTATION: get the device info (if it works then the host can see the device, so return the end state). Otherwise return State::Fail.</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Reformat

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, _scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: reformat a disk and return an end state</p>
<p>PARAMETERS: to_state – the end state to return if reformating is successful</p>
<blockquote>
<p>device – the device information needed to reformat the drive</p>
<p>_scsi_info – this parameter is unused</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after reformatting the drive</p>
<p>IMPLEMENTATION: ensure the drive is NOT mounted. format the device. And if it works, update the UUID of the block device, by creating a new one, probing for the uuid, and looking up the uuid value, then updated the device id. If this all works, return the end state, otherwise if any of the steps fail return State::Fail.</p></td>
</tr>
</tbody>
</table>

###### Debug

#### Scan

##### Trait Implementations

###### Transition

<table>
<thead>
<tr class="header">
<th>Trait Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>transition(to_state: State, device: &amp;mut BlockDevice, scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;, _simulate: bool) -&gt; State</p>
<p>DESCRIPTION: Scan a drive and return a state</p>
<p>PARAMETERS: to_state – the end state to return if scanning is successful</p>
<blockquote>
<p>device – the device information needed to run a scan</p>
<p>scsi_info – the scsi info needed to runa scan</p>
<p>_simulate –this parameter is currently unused</p>
</blockquote>
<p>RETURNS: State after scanning the drive</p>
<p>IMPLEMENTATION: check if the drive is raid backed. If the .0 raid backed is false, run smart checks on the device. If its okay return the end state else Fail. If raid_backed.0 is true, and the Vendor is Hp, check the scsi_info’s state. If the state is us Running, then return the end state, otherwise State::Fail. For any other Vendor, skip the scanning and just return the input end state.</p></td>
</tr>
</tbody>
</table>

###### Debug

### Functions

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>filter_disks(devices: &amp;[PathBuf], storage_detail_id: u32) -&gt; BynarResult&lt;Vec&lt;BlockDevice&gt;&gt;</p>
<p>DESCRIPTION: filter disks and get their information while skipping devices that should not be changed, removed etc, such as loopback, LVM, CD/DVD rom, RAM, root, and BOOT</p>
<p>PARAMETERS: devices – list of devices</p>
<blockquote>
<p>storage_detail_id – the id of some storage detail entry in the database</p>
</blockquote>
<p>RETURNS: Ok(block devices matching the criteria) on success, Error otherwise</p>
<p>IMPLEMENTATION: get a list of all device info from the list of devices, removing all devices where information cannot be looked up. For each device left, check the mount point and partition information and update them accordingly. Then, filter the devices by removing all loopback devices, LVM devices, CD/DVD rom devices, RAM devices, and skip the root disk</p></td>
</tr>
<tr class="even">
<td><p>add_previous_devices(devices: &amp;mut Vec&lt;BlockDevice&gt;, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Add in any disks that the database new about that linux can no longer find</p>
<p>PARAMETERS: devices – list of devices</p>
<blockquote>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the details on how hosts are mapped</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get a list of devices from the database. Add back in any missing devices to the vector. Since the host doesn’t know about the device, we check if the device is waiting for repair. If not, then the database things the disk is still good, so set it as waiting for repair so someone goes and replaces it.</p></td>
</tr>
<tr class="odd">
<td><p>check_all_disks(host_info: &amp;Host, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping) -&gt; BynarResult&lt;Vec&lt;BynarResult&lt;StateMachine&gt;&gt;&gt;</p>
<p>DESCRIPTION: retrieves a list of disks and sets up a state machine on each of them. Retrieves previous state and runs through the state machine and preserves the final state in the database before returning a vector of StateMachine</p>
<p>PARAMETERS: host_info – information on the host</p>
<blockquote>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the details on how hosts are mapped</p>
</blockquote>
<p>RETURNS: Ok( a vector of state machines ) on success, Error otherwise</p>
<p>IMPLEMENTATION: get a list of all devices currently attached to the tree (does not show dead and disconnected devices that are still mounted in /etc/mtab and their scsi information. get the information on all currently mounted devices and remove the ones that we already have, which gives us all broken mounted devices. Get all the info on all devices and filter them, then add any previous devices. Add filtered devices to the database and create a state machine per device. run each state machine and save the state to the database then the run finishes. return a list of state machine end states, otherwise error out if not successful.</p></td>
</tr>
<tr class="even">
<td><p>check_filesystem(filesystem_type: &amp;FilesystemType, device: &amp;Path) -&gt; BynarResult&lt;Fsck&gt;</p>
<p>DESCRIPTION: check if the filesystem on a device is corrupt, given the device and filesystem type. Note this assumes that the device is unmounted</p>
<p>PARAMETERS: filesystem_type – the type of filesystem on the device</p>
<blockquote>
<p>device – the device to check the filesystem on</p>
</blockquote>
<p>RETURNS: Ok(Fsck result ) on success, Error otherwise</p>
<p>IMPLEMENTATION: match the filesystem type and run the correct check function, returning the result.</p></td>
</tr>
<tr class="odd">
<td><p>repair_filesystem(filesystem_type: &amp;FilesystemType, device: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: repair a filesystem, given a corrupted device and filesystem type. Note, this assumes the filesystem is corrupt and the device is unmounted</p>
<p>PARAMETERS: filesystem_type – the type of filesystem on the device</p>
<blockquote>
<p>device – the device to repair the filesystem on</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: match the filesystem type and run the correct repair function, returning the result.</p></td>
</tr>
<tr class="even">
<td><p>check_writable(path: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: check if the path is writable</p>
<p>PARAMETERS: path – the path to check</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: create a temporary directory in the path, and a temporary file in the path. Try writing to the file. If successful, return Ok(()), else error out. Note that the TempDir is deleted once the function exits</p></td>
</tr>
<tr class="odd">
<td><p>check_lvm(device: &amp;Path) -&gt; BynarResult&lt;Fsck&gt;</p>
<p>DESCRIPTION: check if the lvm on a device is corrupt, given the device. Note this assumes that the device is unmounted</p>
<p>PARAMETERS: device – the device to check the lvm on</p>
<p>RETURNS: Ok(Fsck result ) on success, Error otherwise</p>
<p>IMPLEMENTATION: scan the LVM on the device. get the list of volume groups, and check if the physical volumes in each volume group is accessible</p></td>
</tr>
<tr class="even">
<td><p>check_xfs(device: &amp;Path) -&gt; BynarResult&lt;Fsck&gt;</p>
<p>DESCRIPTION: check if the xfs filesystem on a device is corrupt.</p>
<p>PARAMETERS: device – the device to check the xfs filesystem on</p>
<p>RETURNS: Ok(Fsck result ) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the xfs_repair command with the –n flag, which means it will not modify the filesystem but scan it for corruption. If successful, match the error code to Ok, Corrupt, or a fail code. Return the result or error out.</p></td>
</tr>
<tr class="odd">
<td><p>repair_xfs(device: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: repair an xfs filesystem, given a corrupted device Note, this assumes the filesystem is corrupt and the device is unmounted</p>
<p>PARAMETERS: device – the device to repair the filesystem on</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the xfs_repair command on the device path. If successful return Ok(()), else error out.</p></td>
</tr>
<tr class="even">
<td><p>check_ext(device: &amp;Path) -&gt; BynarResult&lt;Fsck&gt;</p>
<p>DESCRIPTION: check if the ext filesystem on a device is corrupt.</p>
<p>PARAMETERS: device – the device to check the ext filesystem on</p>
<p>RETURNS: Ok(Fsck result ) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the e2fsk command with the –n flag, which means it will not modify the filesystem but scan it for corruption. If successful, match the error code to Ok, Corrupt, or a fail code. Return the result or error out.</p></td>
</tr>
<tr class="odd">
<td><p>repair_ext(device: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: repair an ext filesystem, given a corrupted device Note, this assumes the filesystem is corrupt and the device is unmounted</p>
<p>PARAMETERS: device – the device to repair the filesystem on</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: run the e2fsck command with the –p flag and the device path. If successful return Ok(()), else error out.</p></td>
</tr>
<tr class="even">
<td><p>run_smart_checks(device: &amp;Path) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: run smart checks against the disk</p>
<p>PARAMETERS: device – the device to repair the filesystem on</p>
<p>RETURNS: Ok(bool) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the handle for the libatasmart. Get the smart status. If successful, return Ok(true) if the smart status is true, Ok(false) if the smart status is false, otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>format_device(device: &amp;Path) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: format the device with the proper filesystem type.</p>
<p>PARAMETERS: device – the device to format</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the filesystem type of the device. If the filesystem type is Xfs then it needs to be forced to overwrite. format the block device. If successful, return Ok(()), else error out</p></td>
</tr>
<tr class="even">
<td><p>is_device_mounted(dev_path: &amp;Path) -&gt; bool</p>
<p>DESCRIPTION: check if the device is mounted</p>
<p>PARAMETERS: dev_path – the device to check is mounted</p>
<p>RETURNS: true if mounted, false if not</p>
<p>IMPLEMENTATION: first check if the device is mounted, if true then return. otherwise, check if any of the partitions contained in the device are mounted. If so, then return true, otherwise return false</p></td>
</tr>
<tr class="odd">
<td><p>is_disk_blank(dev: &amp;Path) -&gt; BynarResult&lt;bool&gt;</p>
<p>DESCRIPTION: make a best guess as to whether a disk is blank</p>
<p>PARAMETERS: dev – the device to check if blank</p>
<p>RETURNS: Ok(is blank?) else Error</p>
<p>IMPLEMENTATION: open and scan the LVM. Got each volume group, check if there is a PVM with the same name as the input device. If so, return Ok(false). check if the GPT header is readable. If so, return Ok(false). If the device is currently mounted, return true. Attempt to mount the device to a temporary directory, and if successful return false, otherwise it’s probably blank. If successful return Ok(false) for passing tests, Ok(true) if none of the tests pass, and therefore the disk is probably blank</p></td>
</tr>
<tr class="even">
<td><p>is_raid_backed(scsi_info: &amp;Option&lt;(ScsiInfo, Option&lt;ScsiInfo&gt;)&gt;) -&gt; (bool, Vendor)</p>
<p>DESCRIPTION: check if the disk is RAID backed</p>
<p>PARAMETERS: scsi_info – the scsi information to check</p>
<p>RETURNS: (is raid backed?, Who is the Vendor?)</p>
<p>IMPLEMENTATION: if both input option values are None (the outer and inner ScsiInfo), then return (false, Vendor::NONE), otherwise, check if the scsi_type is StorageArray and Enclosure. If so, then check the vendor. If the vendor is Hp, then it is RAID backed, so return (true, Vendor::Hp), otherwise return (false, vendor).</p></td>
</tr>
</tbody>
</table>

# Hardware Testing

## Introduction

Hardware testing module, this module uses the libredfish to check the
Hardware status.

## Hardware Tests

### Struct

#### HardwareHealthSummary

A summary of all the hardware status information

##### Attributes

| Name                | Type                     | Description                              |
| ------------------- | ------------------------ | ---------------------------------------- |
| array\_controllers  | Vec\<BynarResult\<()\>\> | Status of the array controllers          |
| disk\_drives        | Vec\<BynarResult\<()\>\> | A list of the physical disk drive status |
| manager             | Vec\<BynarResult\<()\>\> | The iLo status                           |
| power               | Vec\<BynarResult\<()\>\> | The Power supply status                  |
| storage\_enclosures | Vec\<BynarResult\<()\>\> | Status of the storage enclosures         |
| thermals            | Vec\<BynarResult\<()\>\> | The fan status                           |

### Functions

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>collect_redfish_info(config: &amp;ConfigSettings) -&gt; BynarResult&lt;HardwareHealthSummary&gt;</p>
<p>DESCRIPTION: collect the hardware health information from redfish</p>
<p>PARAMETERS: config – the configuration settings needed to connect to redfish</p>
<p>RETURNS: Ok(hardware health summary) on success, Error otherwise</p>
<p>IMPLEMENTATION: Build a Client socket. If the redfish ip address is not specified, skip that check. Otherwise, parse the redfish configuration from the config input. Create a new instance of a Redfish handler, and get the array controllers. For each controller, add it to the list of controllers and get all storage enclosures attached to the controller and grab all disks attached to the controller. On the resulting lists, evaluate the storage and collect the results into vectors. Get the manager from the redfish handler and evaluate it, same with the thermal and power. If everything is successful, return Ok(HardwareHealthSummary) otherwise error out.</p></td>
</tr>
<tr class="even">
<td><p>check_hardware(config: &amp;ConfigSettings) -&gt; BynarResult&lt;HardwareHealthSummary&gt;</p>
<p>DESCRIPTION: public wrapper function for collect_redfish_info</p>
<p>PARAMETERS: config – the configuration settings needed to connect to redfish</p>
<p>RETURNS: Ok(hardware health summary) on success, Error otherwise</p>
<p>IMPLEMENTATION: call collect_redfish_info and return the result</p></td>
</tr>
<tr class="odd">
<td><p>evaluate_storage&lt;T&gt;(hardware: T) -&gt; BynarResult&lt;()&gt; where T: Hardware + Status</p>
<p>DESCRIPTION: evaluate an input hardware object of type Hardware + Status</p>
<p>PARAMETERS: hardware – the hardware to evaluate</p>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Check if the hardware health is NOT OK. If so, return an Error, otherwise return Ok(())</p></td>
</tr>
<tr class="even">
<td><p>evaluate_manager(manager: &amp;Manager) -&gt; Vec&lt;BynarResult&lt;()&gt;&gt;</p>
<p>DESCRIPTION: evaluate the ilo status. If there are no issues then the vector should be empty</p>
<p>PARAMETERS: manager – the ilo status manager</p>
<p>RETURNS: Vec&lt;is the status of an ilo okay?&gt;</p>
<p>IMPLEMENTATION: look through all the self test results. If the result status is not OK or Informational, then there is an error, so add an Error to the vector of results. return the vector.</p></td>
</tr>
<tr class="odd">
<td><p>evaluate_power(power: &amp;Power) -&gt; Vec&lt;BynarResult&lt;()&gt;&gt;</p>
<p>DESCRIPTION: evaluate the power status. If there are no issues then the vector should be empty</p>
<p>PARAMETERS: power – the Power status</p>
<p>RETURNS: Vec&lt;is the status of a power supply okay?&gt;</p>
<p>IMPLEMENTATION: look through all the self test results. If the result status is not OK then the power supply failed, so add an Error to the vector of results. return the vector.</p></td>
</tr>
<tr class="even">
<td><p>evaluate_power(power: &amp;Power) -&gt; Vec&lt;BynarResult&lt;()&gt;&gt;</p>
<p>DESCRIPTION: evaluate the power status. If there are no issues then the vector should be empty</p>
<p>PARAMETERS: power – the Power status</p>
<p>RETURNS: Vec&lt;is the status of a power supply okay?&gt;</p>
<p>IMPLEMENTATION: look through all the power supply status. If the result status is not OK then the power supply failed, so add an Error to the vector of results. return the vector.</p></td>
</tr>
<tr class="odd">
<td><p>evaluate_thermals(thermal: &amp;Thermal) -&gt; Vec&lt;BynarResult&lt;()&gt;&gt;</p>
<p>DESCRIPTION: evaluate the fan status as well as the temperature. If there are no issues then the vector should be empty</p>
<p>PARAMETERS: thermal – the Thermal status</p>
<p>RETURNS: Vec&lt;is the status of a fan okay?&gt;</p>
<p>IMPLEMENTATION: look through all the fans. If the result status is not OK then the fan failed, so add an Error to the vector of results. look through all the temperature readings, if the health is not OK, then the temperature reading is failing, so add an Error to the vector of results. return the vector.</p></td>
</tr>
</tbody>
</table>

# Bynar

## Introduction

This program handles detection of failed hard drives, files a ticket for
a datacenter technician to replace the drive, waits for the resolution
of the ticket and then makes an API call to the disk-manager to add the
new disk back into the server.

### Main Process Functions

<table>
<thead>
<tr class="header">
<th>Function Definition</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td><p>notify_slack(config: &amp;ConfigSettings, msg: &amp;str) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Send a slack notification</p>
<p>PARAMETERS: config – the configuration settings needed to connect to a Slack channel</p>
<blockquote>
<p>msg – the message to send</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: Create a Slack handle, and connect to the Slack API system using the information in the config. Create the payload to send to the Slack channel, and send it. If successful, return Ok(()), otherwise error out</p></td>
</tr>
<tr class="even">
<td><p>get_public_key(config: &amp;ConfigSettings, host_info: &amp;Host) -&gt; BynarResult&lt;String&gt;</p>
<p>DESCRIPTION: get the server public key if possible from either Hashicorp vault or some .pem file.</p>
<p>PARAMETERS: config – the configuration settings needed to connect to the Hashicorp Vault</p>
<blockquote>
<p>host_info – the host information</p>
</blockquote>
<p>RETURNS: Ok(public key) on success, Error otherwise</p>
<p>IMPLEMENTATION: check if the vault endpoint and token are set. If so, attempt to get the key from the vault using the get_vault_token method. otherwise, check for a .pem file in /etc/bynar for the public key. If successful, return the key, otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>check_for_failed_disks(config: &amp;ConfigSettings, host_info: &amp;Host, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Run a scan to check for failed disks</p>
<p>PARAMETERS: config – the configuration settings needed</p>
<blockquote>
<p>host_info – the host information</p>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the mapping of host details</p>
<p>simulate – passed to skip evaluation</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the public key. Create the description needed to open a ticket in case a disk failed. Check all the drives, running check_all_disks and for each disk check if the state is Waiting For Replacement. If so, add to the description and check if the database knows the disk is waiting for repair. If so, move on to the next disk, otherwise ask the disk-manager if the disk is safe to remove. If yes, notify slack and remove the disk. If false, notify slack and file a ticket with Jira and update the database. If a disk is stuck in the fail state, error message. Otherwise the state is Good so move on. If successful for all disks, return Ok(()), otherwise error out</p></td>
</tr>
<tr class="even">
<td><p>evaluate(results: Vec&lt;BynarResult&lt;()&gt;&gt;, config: &amp;ConfigSettings, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: check if there are any errors besides hardware errors</p>
<p>PARAMETERS: results – the list of errors after running a hardware scan</p>
<blockquote>
<p>config – the configuration settings needed</p>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the mapping of host details</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: for each result, check if the error type is a HardwareError type. If so, check if the hardware is waiting for repair. If it isn’t, file a ticket. If the Error Type is NOT a HardwareError type, then Error out.</p></td>
</tr>
<tr class="odd">
<td><p>check_for_failed_hardware(config: &amp;ConfigSettings, host_info: &amp;Host, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: Run a scan to check for failed hardware</p>
<p>PARAMETERS: config – the configuration settings needed</p>
<blockquote>
<p>host_info – the host information</p>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the mapping of host details</p>
<p>simulate – passed to skip evaluation</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the public key. Create the description needed to open a ticket in case there is a hardware failure. Run check_hardware and evaluate the results disk drives, manager, power, storage enclosures, and thermals for any errors that aren’t Hardware errors. If successful, return Ok(()), otherwise error out.</p></td>
</tr>
<tr class="even">
<td><p>add_repaired_disks(config: &amp;ConfigSettings, host_info: &amp;Host, pool: &amp;Pool&lt;ConnectionManager&gt;, host_mapping: &amp;HostDetailsMapping, simulate: bool) -&gt; BynarResult&lt;()&gt;</p>
<p>DESCRIPTION: add repaired disks back to the cluster</p>
<p>PARAMETERS: config – the configuration settings needed</p>
<blockquote>
<p>host_info – the host information</p>
<p>pool – the pool of connections to the database</p>
<p>host_mapping – the mapping of host details</p>
<p>simulate – passed to skip evaluation</p>
</blockquote>
<p>RETURNS: Ok(()) on success, Error otherwise</p>
<p>IMPLEMENTATION: get the public key. get the list of outstanding repair tickets and check them for resolved tickets. For each ticket in the list, check if the ticket is resolved. If so, connect to the disk-manager and ask it to add the disk back to the cluster. If successful, resolve the ticket in the database. If the ticket is not resolved, skip it. If everything is successful, return Ok(()), otherwise error out.</p></td>
</tr>
<tr class="odd">
<td><p>main()</p>
<p>DESCRIPTION: The main Bynar program, it gathers a list of all disks, checks each disk and decides if it needs replacing and files a ticket. It then records the ticket, checks for resolved tickets and adds the disks back in records everything in the database</p>
<p>PARAMETERS: None</p>
<p>RETURNS: None</p>
<p>IMPLEMENTATION: Create and get the command line arguments. Bynar specifically takes the optional configdir argument, the optional simulate argument, and the optional –v argument. Depending on the number of input –v flags, increase the verbosity of the logs. Check the configdir input and whether the command is a simulation or not. Gather the host information and load the configuration. Connect to the database and update the host information. Check for failed disks, failed hardware, and add in any repaired disks.</p></td>
</tr>
</tbody>
</table>
