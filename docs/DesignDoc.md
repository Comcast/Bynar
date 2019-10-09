## Bynar Documentation

# Revision History

| Name           | Date      | Reason for Change                                                          | Version |
| -------------- | --------- | -------------------------------------------------------------------------- | ------- |
| Michelle Zhong | 10/8/2019 | Outline the Document                                                       | 0.1     |
| Michelle Zhong | 10/9/2019 | Outline the Document Modules, fill in the API section, Config File section | 0.2     |
|                |           |                                                                            |         |
|                |           |                                                                            |         |
# API

## Introduction

This package uses Protobuf version 2 to create Messages that can be sent over the network.  Protobuf is a fast and small protocol for serializing structs (or structured data).  Serialized messages can be sent between Sockets, unpackaged, and read easily and quickly.  The protobuf package automatically generates the rust code needed to create, modify, and destroy Messages as well as their attributes.

## List of Message Enums

### DiskType

The type of disk or device

#### Enum Values

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

### ResultType

A result value

#### Enum Values

| Name | Description       |
| ---- | ----------------- |
| OK   | ok                |
| ERR  | There is an error |

### Op

An operation on a disk

#### Enum Values

| Name              | Description                                             |
| ----------------- | ------------------------------------------------------- |
| Add               | Generic Add Disk command, returns an OpResult           |
| AddPartition      | Add a Partition Command, returns an OpResult            |
| List              | List the Disks, returns a list of Disks                 |
| Remove            | Remove a Disk, returns an OpResult                      |
| SafeToRemove      | Checks if a Disk is safe to remove, returns a bool      |
| GetCreatedTickets | list created tickets, returns a list of created tickets |

### DatacenterOp

Datacenter API&#39;s, these all require server\_id as a parameter for the operation

#### Enum Values

| Name         | Description                                              |
| ------------ | -------------------------------------------------------- |
| GetDc        | Get ? Returns an OpStringResult                          |
| GetRack      | Get the rack of a server, returns an OpStringResult      |
| GetRow       | Get the row of a server, returns an OpStringResult       |
| GetElevation | Get the elevation of a server, returns an OpStringResult |

## List of Message Structs

### Osd

A Ceph OSD object descriptor

#### Attributes

| Name          | Type           | Description                                 |
| ------------- | -------------- | ------------------------------------------- |
| fsid          | Option(String) | OSD File System ID, if one exists           |
| id            | u64            | OSD ID number                               |
| block\_device | String         | Block Device of the OSD                     |
| journal       | Option(String) | Name of the Journal if the OSD has one set  |
| active        | bool           | Whether or not an OSD is active or a spare  |
| used\_space   | u64            | How much space in the OSD is currently used |
| total\_space  | u64            | Total space in the OSD                      |

### Partition

A single partition descriptor

#### Attributes

| Name       | Type   | Description                                      |
| ---------- | ------ | ------------------------------------------------ |
| uuid       | String | The id of the partition                          |
| first\_lba | u64    | The first logical block address of the partition |
| last\_lba  | u64    | The last logical block address of the partition  |
| flags      | u64    | Flags associated with the partition              |
| name       | String | The name of the partition                        |

### PartitionInfo

A list of Partitions

#### Attributes

| Name      | Type                   | Description        |
| --------- | ---------------------- | ------------------ |
| partition | Vec&lt;Partition&gt; | List of partitions |

### Disk

A disk object descriptor

#### Attributes

| Name           | Type                   | Description        |
| -------------- | ---------------------- | ------------------ |
| type           | DiskType               | The type of disk   |
| dev\_path      | String                 | ?? Device path?    |
| partitions     | PartitionInfo          | Disk partitions    |
| serial\_number | Option&lt;String&gt; | Disk serial number |

### OpResult

A result of an Op message

#### Attributes

| Name       | Type                   | Description                        |
| ---------- | ---------------------- | ---------------------------------- |
| result     | ResultType             | Whether the result is ok or Error  |
| error\_msg | Option&lt;String&gt; | Error message if there is an error |

### OpBoolResult

A boolean result of an Op message

#### Attributes

| Name       | Type                   | Description                               |
| ---------- | ---------------------- | ----------------------------------------- |
| result     | ResultType             | Whether Ok or Error                       |
| value      | Option&lt;bool&gt;   | A value is set if OK                      |
| error\_msg | Option&lt;String&gt; | Error message is set if there is an Error |

### OpStringResult

A String result of an Op message

#### Attributes

| Name       | Type                   | Description                               |
| ---------- | ---------------------- | ----------------------------------------- |
| result     | ResultType             | Whether Ok or Error                       |
| value      | Option&lt;String&gt; | A value is set if OK                      |
| error\_msg | Option&lt;String&gt; | Error message is set if there is an Error |

### JiraInfo

A Jira Ticket information descriptor

#### Attributes

| Name         | Type   | Description          |
| ------------ | ------ | -------------------- |
| ticket\_id   | String | Ticket number        |
| server\_name | String | A value is set if OK |

### OpJiraTicketsResult

A Jira ticket result

#### Attributes

| Name       | Type                   | Description                               |
| ---------- | ---------------------- | ----------------------------------------- |
| result     | ResultType             | Whether Ok or Error                       |
| value      | Option&lt;String&gt; | A value is set if OK                      |
| error\_msg | Option&lt;String&gt; | Error message is set if there is an Error |

### DatacenterOperation

A Datacenter operation message

#### Attributes

| Name       | Type         | Description                            |
| ---------- | ------------ | -------------------------------------- |
| Op\_type   | DatacenterOp | The type of operation to be performed  |
| server\_id | String       | The ID of the server to be operated on |

### Operation

A service operation that can be performed

#### Attributes

| Name             | Type                   | Description                                                                   |
| ---------------- | ---------------------- | ----------------------------------------------------------------------------- |
| Op\_type         | Op                     | The operation type                                                            |
| disk             | Option&lt;String&gt; | The disk name, used for an Add or Remove                                      |
| simulate         | Option&lt;bool&gt;   | Whether the operation is a simulation, used for Add, Remove, and SafeToRemove |
| partition\_start | Option&lt;u64&gt;    | Optional field for AddPartition, start of a partition                         |
| partition\_end   | Option&lt;u64&gt;    | Optional field for AddPartition, end of a partition                           |
| partition\_name  | Option&lt;String&gt; | Optional field for AddPartition, partition name                               |
| osd\_id          | Option&lt;u64&gt;    | Optional Ceph related field, the id of an OSD                                 |
| replica\_set     | Vector&lt;String&gt; | Host:/dev/disk strings list for gluster replica sets                          |

# Configuration Files

## Introduction

Bynar uses a set of configuration files to configure different settings.  Bynar uses JSON as the format for its configuration files, as JSON files are easily parsed, serialized, and deserialized using the Rust serde and serde-json crates.

## List of Config Files

### Bynar JSON

This config file, bynar.json, is used to configure several different settings, including a Slack webhook, JIRA support, Redfish access, Vault password access, and Postgres database access

| Name                   | Description                               | Example Value                                                                          |
| ---------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------- |
| proxy                  | Proxy web server?                         | &quot;https://my.proxy&quot;                                                           |
| manager\_host          | The host ip of the bynar disk manager     | &quot;localhost&quot;                                                                  |
| manager\_port          | The port of the Bynar disk manager        | 5555                                                                                   |
| slack\_webhook         | Slack webhook to access Slack API         | &quot;[https://hooks.slack.com/services/ID](https://hooks.slack.com/services/ID)&quot; |
| slack\_channel         | Slack channel to post messages to         | &quot;#my-channel&quot;                                                                |
| slack\_botname         | Name of the Bot to post messages under    | &quot;my-bot&quot;                                                                     |
| jira\_user             | JIRA username to create tickets under     | &quot;test\_user&quot;                                                                 |
| jira\_password         | JIRA password                             | &quot;user\_pass&quot;                                                                 |
| jira\_host             | JIRA host to create tickets under         | &quot;https://tickets.jira.com&quot;                                                   |
| jira\_issue\_type      | JIRA issue type name to create tickets of | &quot;3&quot;                                                                          |
| jira\_priority         | JIRA priority value of tickets created    | &quot;4&quot;                                                                          |
| jira\_project\_id      | JIRA project id to create tickets under   | &quot;MyProject&quot;                                                                  |
| jira\_ticket\_assignee | User created JIRA tickets are assigned to | &quot;assignee\_username&quot;                                                         |
| redfish\_ip            | IP address of a Redfish instance          | &quot;localhost&quot;                                                                  |
| redfish\_username      | Username to access Redfish instance       | &quot;redfish\_user&quot;                                                              |
| redfish\_password      | Password to access Redfish                | &quot;redfish\_pass&quot;                                                              |
| redfish\_port          | Port of the Redfish instance              | 4443                                                                                   |
| vault\_endpoint        | Hashicorp vault endpoint                  | &quot;https://my\_vault.com&quot;                                                      |
| vault\_token           | Hashicorp vault token to access the vault | &quot;token\_234464562&quot;                                                           |
| database               | List of Database parameters               |                                                                                        |
| database:username      | Username to access database with          | &quot;postgres&quot;                                                                   |
| database:password      | Password to access database with          | &quot;&quot;                                                                           |
| database:port          | Port of the database                      | 5432                                                                                   |
| database:dbname        | Name of the database                      | &quot;bynar&quot;                                                                      |
| database:endpoint      | Database endpoint                         | &quot;some.endpoint&quot;                                                              |

### Ceph JSON

This config file, ceph.json, is used to tell Bynar where the ceph.conf file is, what user to use when running Ceph commands, and
#
[ANNOTATION:

BY &#39;Zhong, Michelle&#39;
ON &#39;2019-10-09T10:56:00&#39;ZM
NOTE: &#39;To be honest I don&#39;t know what this journal\_devices thing is used for&#39;]
what journal devices are known?

| Name                           | Description                            | Example Value                   |
| ------------------------------ | -------------------------------------- | ------------------------------- |
| config\_file                   | The path to the ceph.conf file         | &quot;/etc/ceph/ceph.conf&quot; |
| user\_id                       | User to use when running Ceph commands | &quot;admin&quot;               |
| journal\_devices               | Journal device list                    |                                 |
| journal\_devices:device        | Path of the device                     | &quot;/dev/sda&quot;            |
| journal\_devices:partition\_id | Partition ID number                    | 1                               |

### Disk-Manager JSON

This config file, disk-manager.json is used to tell Bynar what the backend storage system is

| Name    | Description                            | Example Value    |
| ------- | -------------------------------------- | ---------------- |
| backend | The backend type of the storage system | &quot;ceph&quot; |

# Backend

## Introduction

### Backend Module

### Ceph

# Database Schema

## Introduction

### Postgres

# Database Logging

## Introduction

### Logging

# Helper Functions

## Introduction

### Error Module

### Host Information

### Helper Module

# Client

## Introduction

### Client Interface

# Support Tickets

## Introduction

### JIRA Support

# Disk Manager

## Introduction

### Disk Manager

# Disk Testing

## Introduction

### State Machine

# Hardware Testing

## Introduction

### Hardware Tests

# Bynar

## Introduction

### Main Process