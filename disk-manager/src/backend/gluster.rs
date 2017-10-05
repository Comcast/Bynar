extern crate gluster;
extern crate mktemp;

use backend::Backend;

use self::mktemp::Temp;

pub struct GlusterBackend;
/*
    Add a disk
    0. Make sure the brick pid isn't running
    1. Need to know which brick we're replacing.
      a. Pull from sqlite3
    2. Create a dir on the gluster mount that doesn't exist
    Using Temp
         mkdir /mnt/r2/<name-of-nonexistent-dir>
         rmdir /mnt/r2/<name-of-nonexistent-dir>
         setfattr -n trusted.non-existent-key -v abc /mnt/r2
         setfattr -x trusted.non-existent-key  /mnt/r2

    Remove a disk
    1. Kill the brick pid
    2. wipe it
    3. Record in sqlite where it was mounted so we can replace it
    later
*/

impl Backend for GlusterBackend {
    fn add_disk(&self, device: &Path, simulate: bool) -> IOResult<()> {
        //self.add_osd(device, simulate).map_err(|e| {
        //   Error::new(ErrorKind::Other, e)
        //})?;
        Ok(())
    }
    fn remove_disk(&self, device: &Path, simulate: bool) -> IOResult<()> {
        //self.remove_osd(device, simulate).map_err(|e| {
        //    Error::new(ErrorKind::Other, e)
        //})?;
        Ok(())
    }

    fn safe_to_remove(&self, device: &Path, simulate: bool) -> IOResult<bool> {
        Ok(true)
    }
}
