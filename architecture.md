All API communication happens over TCP+Protobuf 
```
                                              +-------------------------------------+
+--------------------------+  Add Disk        |      Disk-Manager                   |
| Parallel-Deploy          +----------------> |                                     |
|                          |                  |Runs on servers.  Waits for requests |
| Very quickly setup       |            +----^+                                     |
| thousands of ceph osds   |            |     +---------------------------+---------+
+--------------------------+            |                                 ^
                                        |                                 | Remove Disk
                                        |                                 | Add Disk
                                        |                                 |
                                        |Add Disk                         |
                                        |Remove Disk     +----------------+--------------------------------+
                                        |List Disks      |    Dead-Disk-Detector                           |
                        Manual API calls|                |                                                 |
                                        |                | Looks for bad drives                            |
                                        |                | Removes drives from cluster                     |
                          +-------------+---+            | Creates support tickets in jira                 |
                          |   Client        |            | Puts drives back into cluster after resolution  |
                          |                 |            |                                                 |
                          | List disks      |            +-------------------------------------------------+
                          | Add disk        |
                          | Remove disk     |
                          |                 |
                          +-----------------+

```
