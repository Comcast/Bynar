CREATE TABLE IF NOT EXISTS process_manager (
        entry_id SERIAL NOT NULL UNIQUE,
        pid INTEGER NOT NULL, -- pid of daemon
        ip VARCHAR NOT NULL, -- ip where daemon is running
        status VARCHAR(20), -- status of the daemon
        start_time TIMESTAMPTZ NOT NULL DEFAULT now(),-- epoch when it started
        PRIMARY KEY (ip,pid)
        );


CREATE TABLE IF NOT EXISTS regions (
    region_id SERIAL NOT NULL UNIQUE,
    region_name VARCHAR (256) PRIMARY KEY NOT NULL 
    );

CREATE TABLE IF NOT EXISTS storage_types (
    storage_id SERIAL NOT NULL UNIQUE,
    storage_type VARCHAR (256) PRIMARY KEY NOT NULL
    );

INSERT INTO storage_types (storage_type) vALUES ('ceph');
INSERT INTO storage_types (storage_type) vALUES ('sio');
INSERT INTO storage_types (storage_type) vALUES ('solidfire');
INSERT INTO storage_types (storage_type) VALUES ('hitachi');

CREATE TABLE IF NOT EXISTS storage_details (
    detail_id SERIAL NOT NULL UNIQUE,
    storage_id INTEGER REFERENCES storage_types(storage_id) ON DELETE CASCADE,
    region_id INTEGER REFERENCES regions(region_id) ON DELETE CASCADE,
    hostname VARCHAR (512),
    name_key1 VARCHAR, -- name of storage array
    uuid VARCHAR,
    name_key2 VARCHAR,-- name of pool, switch etc
    PRIMARY KEY (region_id, storage_id, hostname, name_key1, name_key2)
    );

CREATE TABLE IF NOT EXISTS disks (
    disk_id SERIAL NOT NULL UNIQUE,
    disk_uuid VARCHAR,
    detail_id INTEGER REFERENCES storage_details(detail_id) ON DELETE CASCADE,
    disk_name VARCHAR,
    disk_path VARCHAR NOT NULL,
    UNIQUE (disk_path, detail_id)
    );

CREATE TABLE IF NOT EXISTS operation_types (
    type_id SERIAL NOT NULL UNIQUE,
    op_name VARCHAR (128) PRIMARY KEY NOT NULL
    );

INSERT INTO operation_types (op_name) VALUES ('diskadd');
INSERT INTO operation_types (op_name) VALUES ('diskreplace');
INSERT INTO operation_types (op_name) VALUES ('diskremove');
INSERT INTO operation_types (op_name) VALUES ('clusteradd');
INSERT INTO operation_types (op_name) VALUES ('clusterdelete');
INSERT INTO operation_types (op_name) VALUES ('waitforreplacement');
-- Evaluation combines all the internal work like checking 
-- file system for corruption, attempting repair etc.
INSERT INTO operation_types (op_name) VALUES ('evaluation');


CREATE TABLE IF NOT EXISTS operations (
    operation_id SERIAL NOT NULL UNIQUE,
    region_id INTEGER REFERENCES regions(region_id) ON DELETE CASCADE,
    storage_detail_id INTEGER REFERENCES storage_details(detail_id) ON DELETE CASCADE,
    disk_id INTEGER REFERENCES disks(disk_id) ON DELETE CASCADE,
    entry_id INTEGER REFERENCES process_manager(entry_id), -- do not delete cascade
    -- this record is still needed after bynar stops running on a system
    start_time TIMESTAMPTZ NOT NULL,-- when operation started
    snapshot_time TIMESTAMPTZ, -- when last updated
    done_time TIMESTAMPTZ, --  when operation is done
    behalf_of VARCHAR(256), -- who requested this
    reason VARCHAR
    );

CREATE TABLE IF NOT EXISTS operation_details (
    operation_detail_id SERIAL NOT NULL UNIQUE,
    operation_id INTEGER REFERENCES operations(operation_id) ON DELETE CASCADE,
    type_id INTEGER REFERENCES operation_types(type_id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL, -- one of pending, in-progress, done
    tracking_id VARCHAR, -- JIRA tracking id
    start_time TIMESTAMPTZ NOT NULL,-- when it started
    snapshot_time TIMESTAMPTZ, -- when last updated
    done_time TIMESTAMPTZ, -- when operation is done
    PRIMARY KEY (operation_id, type_id, status)
);
