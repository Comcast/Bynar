# ceph_dead_disk
Dead disk management for Ceph

This will be a disk replacement automation pipeline. There's several steps that used to require a human that 
I would like to cut out:
1. Identify bad disks
2. File a ticket request with JIRA
3. Keep track of the file request in a local sqlite database
4. Watch for JIRA ticket resolution
5. Insert the disk back into the Ceph cluster
