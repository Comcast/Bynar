# Bynar Documentation

# Revision History

| Name           | Date       | Reason for Change    | Version |
| -------------- | ---------- | -------------------- | ------- |
| Michelle Zhong | 10/18/2019 | Outline the Document | 0.1     |
|                |            |                      |         |
|                |            |                      |         |
|                |            |                      |         |

# API

## Introduction

This package uses Protobuf version 2 to create Messages that can be sent over the network.  Protobuf is a fast and small protocol for serializing structs (or structured data).  Serialized messages can be sent between Sockets, unpackaged, and read easily and quickly.  The protobuf package generates the rust code needed to create, modify, and destroy Messages as well as their attributes.

## List of Message Enums

### DiskType

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

| Enum Values |
| --- |
| Name | Description |
| OK | ok |
| ERR | There is an error |

## List of Message Structs

### Osd

| Attributes |
| --- |
| Name | Type | Description |
| fsid | Option(String) | OSD File System ID, if one exists |
| id | u64 | OSD ID number |
| block\_device | String | Block Device of the OSD |
| journal | Option(String) | Name of the Journal if the OSD has one set |
| active | bool | Whether or not an OSD is active or a spare |
| used\_space | u64 | How much space in the OSD is currently used |
| total\_space | u64 | Total space in the OSD |

### Partition

| Attributes |
| --- |
| Name | Type | Description |
| uuid | String | The id of the partition |
| first\_lba | u64 | The first logical block address of the partition |
| last\_lba | u64 | The last logical block address of the partition |
| flags | u64 | Flags associated with the partition |
| name | String | The name of the partition |

### PartitionInfo

| Attributes |
| --- |
| Name | Type | Description |
| partition | Vec\&lt;Partition\&gt; | List of partitions |

### Disk

| Attributes |
| --- |
| Name | Type | Description |
| type | DiskType | The type of disk |
| dev\_path | String | ?? Device path? |
| partitions | PartitionInfo | Disk partitions |
| serial\_number | Option\&lt;String\&gt; | Disk serial number |

### OpResult

| Attributes |
| --- |
| Name | Type | Description |
| result | ResultType | Whether the result is ok or Error |
| error\_msg | Option\&lt;String\&gt; | Error message if there is an error |

### OpBoolResult

| Attributes |
| --- |
| Name | Type | Description |
| result | ResultType | Whether Ok or Error |
| value | Option\&lt;bool\&gt; | A value is set if OK |
| error\_msg | Option\&lt;String\&gt; | Error message is set if there is an Error |

### OpStringResult

| Attributes |
| --- |
| Name | Type | Description |
| result | ResultType | Whether Ok or Error |
| value | Option\&lt;String\&gt; | A value is set if OK |
| error\_msg | Option\&lt;String\&gt; | Error message is set if there is an Error |

### JiraInfo

| Attributes |
| --- |
| Name | Type | Description |
| ticket\_id | String | Ticket number |
| server\_name | String | A value is set if OK |