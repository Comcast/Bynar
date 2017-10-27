All API communication happens over TCP+Protobuf 
```
                                              +-------------------------------------+
                                              |      Disk-Manager                   |
                                              |                                     |
                                              |Runs on servers.  Waits for requests |
                                        +----^+                                     |
                                        |     +---------------------------+---------+
                                        |                                 ^
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
